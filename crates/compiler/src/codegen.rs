use std::{
    io::{self, Write},
    sync::{Arc, Mutex},
};

use codegen_macros::{self, bmr_emitter};
use swc_common::{sync::Lrc, SourceMap, Span, Spanned};
use swc_ecma_ast::{
    Expr, JSXAttr, JSXAttrName, JSXAttrOrSpread, JSXAttrValue, JSXClosingElement,
    JSXClosingFragment, JSXElement, JSXElementChild, JSXElementName, JSXEmptyExpr, JSXExpr,
    JSXExprContainer, JSXFragment, JSXMemberExpr, JSXNamespacedName, JSXObject, JSXOpeningElement,
    JSXOpeningFragment, JSXSpreadChild, JSXText,
};
use swc_ecma_codegen::{
    list::ListFormat,
    text_writer::{JsWriter, WriteJs},
    util::SourceMapperExt,
    Emitter as SWCEmitter,
};

use crate::parser::ParseResult;

#[derive(Debug, Default)]
pub struct SharedBuffer(pub Arc<Mutex<Vec<u8>>>);

impl SharedBuffer {
    pub fn clone(&self) -> Self {
        SharedBuffer(Arc::clone(&self.0))
    }
}

impl Write for SharedBuffer {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        self.0.lock().unwrap().write(buf)
    }

    fn flush(&mut self) -> std::io::Result<()> {
        self.0.lock().unwrap().flush()
    }
}

pub type Result = io::Result<()>;

trait BoomerNode: Spanned {
    fn emit_with_bmr<W>(&self, e: &mut Emitter<'_, W>) -> Result
    where
        W: Write;
}

impl<N: BoomerNode> BoomerNode for Box<N> {
    #[inline]
    fn emit_with_bmr<W>(&self, e: &mut Emitter<'_, W>) -> Result
    where
        W: Write,
    {
        (**self).emit_with_bmr(e)
    }
}

impl<'a, N: BoomerNode> BoomerNode for &'a N {
    #[inline]
    fn emit_with_bmr<W>(&self, e: &mut Emitter<'_, W>) -> Result
    where
        W: Write,
    {
        (**self).emit_with_bmr(e)
    }
}

macro_rules! opt_leading_space {
    ($emitter:expr, $e:expr) => {
        if let Some(ref e) = $e {
            formatting_space!($emitter);
            emit!($emitter, e);
        }
    };
}

macro_rules! opt {
    ($emitter:expr, $e:expr) => {{
        if let Some(ref expr) = $e {
            emit!($emitter, expr);
        }
    }};
    ($emitter:expr, $e:expr,) => {{
        opt!($emitter, $e)
    }};
}

macro_rules! emit {
    ($emitter:expr, $e:expr) => {{
        crate::codegen::BoomerNode::emit_with_bmr(&$e, $emitter)?
    }};
}

macro_rules! emit_swc {
    ($emitter:expr, $e:expr) => {
        swc_ecma_codegen::Node::emit_with(&$e, &mut $emitter.swc_emitter)?
    };
}

macro_rules! keyword {
    ($emitter:expr, $span:expr, $s:expr) => {
        $emitter.wr.write_keyword(Some($span), $s)?
    };
    ($emitter:expr, $s:expr) => {
        $emitter.wr.write_keyword(None, $s)?
    };
}

macro_rules! punct {
    ($emitter:expr, $sp:expr, ";") => {
        $emitter.wr.write_semi(Some($sp))?;
    };
    ($emitter:expr, $sp:expr, $s:expr) => {
        $emitter.wr.write_punct(Some($sp), $s)?;
    };

    ($emitter:expr, ";") => {
        $emitter.wr.write_semi(None)?
    };
    ($emitter:expr, $s:expr) => {
        $emitter.wr.write_punct(None, $s)?
    };
}

macro_rules! operator {
    ($emitter:expr, $sp:expr, $s:expr) => {
        $emitter.wr.write_operator(Some($sp), $s)?;
    };

    ($emitter:expr, $s:expr) => {
        $emitter.wr.write_operator(None, $s)?;
    };
}

macro_rules! space {
    ($emitter:expr) => {
        $emitter.wr.write_space()?
    };
    ($emitter:expr,) => {
        space!($emitter)
    };
}

macro_rules! formatting_space {
    ($emitter:expr) => {
        // TODO if !$emitter.cfg.minify {
        $emitter.wr.write_space()?;
        // }
    };
    ($emitter:expr,) => {
        formatting_space!($emitter)
    };
}

/// This macro *may* emit a semicolon, if it's required in this context.
macro_rules! formatting_semi {
    ($emitter:expr) => {
        punct!($emitter, ";")
    };
    ($emitter:expr, ) => {
        punct!($emitter, ";")
    };
}

/// This macro *always* emits a semicolon, as it's required by the structure we
/// emit.
macro_rules! semi {
    ($emitter:expr, $sp:expr) => {
        $emitter.wr.write_semi(Some($sp))?;
    };
    ($emitter:expr) => {
        $emitter.wr.write_semi(None)?;
    };
}

///
/// - `srcmap!(true)` for start (span.lo)
/// - `srcmap!(false)` for end (span.hi)
macro_rules! srcmap {
    ($emitter:expr, $n:expr, true) => {{
        let span = $n.span();
        if !span.is_dummy() {
            $emitter.wr.add_srcmap(span.lo)?;
        }
    }};
    ($emitter:expr, $n:expr, false) => {
        let hi = $n.span().hi;
        if hi != swc_common::BytePos(0) {
            $emitter.wr.add_srcmap(hi)?;
        }
    };
}

pub struct Emitter<'a, W>
where
    W: Write,
{
    swc_emitter: SWCEmitter<'a, JsWriter<'a, W>>,
    wr: JsWriter<'a, W>,
    source_map: Lrc<SourceMap>,
}

impl<'a, W> Emitter<'a, W>
where
    W: Write,
{
    // TODO: sort out this shared buf stuff
    pub fn new(cm: Lrc<SourceMap>, buf0: W, buf1: W) -> Self {
        let swc_emitter = SWCEmitter {
            cfg: swc_ecma_codegen::Config { minify: false },
            cm: cm.clone(),
            comments: None,

            wr: JsWriter::new(cm.clone(), "\n", buf0, None),
        };

        Emitter {
            swc_emitter,
            wr: JsWriter::new(cm.clone(), "\n", buf1, None),
            source_map: cm,
        }
    }

    pub fn emit_bmr_stmts(&mut self, result: ParseResult) -> Result {
        if let Some(server) = result.server {
            // TODO don't unwrap here
            for stmt in server.block.stmts {
                emit_swc!(self, stmt);
            }
        }

        if let Some(jsx) = result.jsx {
            self.emit_bmr_jsx(&jsx)?;
        }

        Ok(())
    }

    pub fn emit_bmr_jsx(&mut self, node: &JSXElement) -> Result {
        self.wr.write_str("export const $$Component = [`")?;
        self.emit_jsx_element(node)?;
        self.wr.write_str("`]")
    }

    #[bmr_emitter]
    fn emit_jsx_element(&mut self, node: &JSXElement) -> Result {
        emit!(node.opening);
        self.emit_jsx_element_or_fragment_children(node.span, &node.children)?;

        if let Some(ref closing) = node.closing {
            emit!(closing)
        }
    }

    #[bmr_emitter]
    fn emit_jsx_opening_element(&mut self, node: &JSXOpeningElement) -> Result {
        punct!("<");
        emit!(node.name);
        space!();

        self.emit_jsx_attributes(node.span, &node.attrs)?;

        if node.self_closing {
            punct!("/");
        }
        punct!(">");
    }

    #[bmr_emitter]
    fn emit_jsx_closing_element(&mut self, node: &JSXClosingElement) -> Result {
        punct!("</");
        emit!(node.name);
        punct!(">");
    }

    #[bmr_emitter]
    fn emit_jsx_element_name(&mut self, node: &JSXElementName) -> Result {
        match *node {
            JSXElementName::Ident(ref n) => emit_swc!(n),
            JSXElementName::JSXMemberExpr(ref n) => emit!(n),
            JSXElementName::JSXNamespacedName(ref n) => emit!(n),
        }
    }

    #[bmr_emitter]
    fn emit_jsx_attr(&mut self, node: &JSXAttr) -> Result {
        emit!(node.name);

        if let Some(ref value) = node.value {
            punct!("=");
            emit!(value);
        }
    }

    #[bmr_emitter]
    fn emit_jsx_attr_value(&mut self, node: &JSXAttrValue) -> Result {
        match *node {
            JSXAttrValue::Lit(ref n) => emit_swc!(n),
            JSXAttrValue::JSXExprContainer(ref n) => emit!(n),
            JSXAttrValue::JSXElement(ref n) => emit!(n),
            JSXAttrValue::JSXFragment(ref n) => emit!(n),
        }
    }

    #[bmr_emitter]
    fn emit_jsx_attr_name(&mut self, node: &JSXAttrName) -> Result {
        match *node {
            JSXAttrName::Ident(ref n) => emit_swc!(n),
            JSXAttrName::JSXNamespacedName(ref n) => emit!(n),
        }
    }

    #[bmr_emitter]
    fn emit_jsx_attr_or_spread(&mut self, node: &JSXAttrOrSpread) -> Result {
        match *node {
            JSXAttrOrSpread::JSXAttr(ref n) => emit!(n),
            JSXAttrOrSpread::SpreadElement(ref n) => {
                punct!("{");
                emit_swc!(n);
                punct!("}");
            }
        }
    }

    #[bmr_emitter]
    fn emit_jsx_element_child(&mut self, node: &JSXElementChild) -> Result {
        match *node {
            JSXElementChild::JSXElement(ref n) => emit!(n),
            JSXElementChild::JSXExprContainer(ref n) => emit!(n),
            JSXElementChild::JSXFragment(ref n) => emit!(n),
            JSXElementChild::JSXSpreadChild(ref n) => emit!(n),
            JSXElementChild::JSXText(ref n) => emit!(n),
        }
    }

    #[bmr_emitter]
    fn emit_jsx_spread_child(&mut self, node: &JSXSpreadChild) -> Result {
        punct!("{");
        punct!("...");
        emit_swc!(node.expr);
        punct!("}");
    }

    #[bmr_emitter]
    fn emit_jsx_expr_container(&mut self, node: &JSXExprContainer) -> Result {
        punct!("`,");
        match &node.expr {
            JSXExpr::JSXEmptyExpr(e) => emit!(e),
            JSXExpr::Expr(expr) => {
                if let Expr::JSXElement(e) = &**expr {
                    emit!(e);
                } else {
                    emit_swc!(expr);
                }
            }
        }
        punct!(",`");
    }

    #[bmr_emitter]
    fn emit_jsx_expr(&mut self, node: &JSXExpr) -> Result {
        match *node {
            JSXExpr::Expr(ref n) => emit_swc!(n),
            JSXExpr::JSXEmptyExpr(ref n) => emit!(n),
        }
    }

    #[bmr_emitter]
    fn emit_jsx_fragment(&mut self, node: &JSXFragment) -> Result {
        emit!(node.opening);
        self.emit_jsx_element_or_fragment_children(node.span, &node.children)?;
        emit!(node.closing);
    }

    #[bmr_emitter]
    fn emit_jsx_opening_fragment(&mut self, node: &JSXOpeningFragment) -> Result {
        punct!("<>")
    }

    #[bmr_emitter]
    fn emit_jsx_closing_fragment(&mut self, node: &JSXClosingFragment) -> Result {
        punct!("</>")
    }

    #[bmr_emitter]
    fn emit_jsx_namespaced_name(&mut self, node: &JSXNamespacedName) -> Result {
        emit_swc!(node.ns);
        punct!(":");
        emit_swc!(node.name);
    }

    #[bmr_emitter]
    fn emit_jsx_empty_expr(&mut self, node: &JSXEmptyExpr) -> Result {}

    #[bmr_emitter]
    fn emit_jsx_text(&mut self, node: &JSXText) -> Result {
        emit_swc!(node);
    }

    #[bmr_emitter]
    fn emit_jsx_member_expr(&mut self, node: &JSXMemberExpr) -> Result {
        emit!(node.obj);
        punct!(".");
        emit_swc!(node.prop);
    }

    #[bmr_emitter]
    fn emit_jsx_object(&mut self, node: &JSXObject) -> Result {
        match *node {
            JSXObject::Ident(ref n) => emit_swc!(n),
            JSXObject::JSXMemberExpr(ref n) => emit!(n),
        }
    }

    fn emit_jsx_element_or_fragment_children(
        &mut self,
        parent_node: Span,
        children: &Vec<JSXElementChild>,
    ) -> Result {
        let format = ListFormat::SingleLine | ListFormat::NoInterveningComments;

        if children.is_empty() {
            return Ok(());
        }

        if self
            .source_map
            .should_write_closing_line_terminator(parent_node, children, format)
        {
            // TODO if minify don't do this
            self.wr.write_line()?;
        }

        // Emit each child.
        let mut previous_sibling: Option<Span> = None;
        for child in children {
            // Write the delimiter if this is not the first node.
            if let Some(previous_sibling) = previous_sibling {
                // Write either a line terminator or whitespace to separate the elements.
                if self.source_map.should_write_separating_line_terminator(
                    Some(previous_sibling),
                    Some(child),
                    format,
                ) {
                    // TODO if minify don't do this
                    self.wr.write_line()?;
                }
            }

            child.emit_with_bmr(self)?;

            previous_sibling = Some(child.span());
        }

        if self
            .source_map
            .should_write_closing_line_terminator(parent_node, children, format)
        {
            // TODO if minify don't do this
            self.wr.write_line()?;
        }

        Ok(())
    }

    fn emit_jsx_attributes(
        &mut self,
        parent_node: Span,
        children: &Vec<JSXAttrOrSpread>,
    ) -> Result {
        let format = ListFormat::SingleLine
            | ListFormat::SpaceBetweenSiblings
            | ListFormat::NoInterveningComments;

        if children.is_empty() {
            return Ok(());
        }

        if self
            .source_map
            .should_write_leading_line_terminator(parent_node, children, format)
        {
            // if !self.cfg.minify {
            self.wr.write_line()?;
            // }
        }

        let mut previous_sibling: Option<Span> = None;
        for child in children {
            // Write the delimiter if this is not the first node.
            if let Some(previous_sibling) = previous_sibling {
                // Write either a line terminator or whitespace to separate the elements.
                if self.source_map.should_write_separating_line_terminator(
                    Some(previous_sibling),
                    Some(child),
                    format,
                ) {
                    // if !self.cfg.minify {
                    self.wr.write_line()?;
                    // }
                } else {
                    formatting_space!(self);
                }
            }

            child.emit_with_bmr(self)?;

            previous_sibling = Some(child.span());
        }

        Ok(())
    }
}

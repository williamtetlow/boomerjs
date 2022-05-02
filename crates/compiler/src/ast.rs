struct BoomerScript {
    root: Element,
}

struct Element {
    pub span: Span,
    pub startTag: StartTag,
    pub children: Vec<ElementChild>,
    pub endTag: Option<EndTag>, // can be self closing <bla/>
}

struct StartTag {
    pub name: JSXElementName,
    pub span: Span,
    pub attrs: Vec<JSXAttrOrSpread>,
    pub self_closing: bool,
    pub type_args: Option<TsTypeParamInstantiation>,
}

struct EndTag {
    pub span: Span,
    pub name: JSXElementName,
}

struct ElementChild {}

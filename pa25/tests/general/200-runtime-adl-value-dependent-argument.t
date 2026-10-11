// A call whose argument's type is a template-id naming a non-type template
// parameter (`I<N - 1>`) is dependent (14.6.2.1p7, 14.6.2p1): in an
// instantiation its argument-dependent lookup sees the instantiation's context
// (14.6.4.2), which declares the template itself, so a trailing return type
// recurses through ADL. Where a non-template candidate is a perfect match the
// templates are not deduced (as GCC and Clang resolve, P3606): depth<0>'s
// return type, which would recurse without end, is never formed

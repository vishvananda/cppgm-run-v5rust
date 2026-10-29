// Reference-temporary traits observe the outer reference conversion, while
// the constructor's scalar parameter conversion remains available separately.
struct A { A(int); };
static_assert(__reference_constructs_from_temporary(const A&, int), "direct temporary");
static_assert(__reference_converts_from_temporary(const A&, int), "copy temporary");

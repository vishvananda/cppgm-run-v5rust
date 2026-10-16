// A semantic assertion diagnoses signed minimum negation in its original type.
static_assert(-(-2147483647 - 1), "overflow");

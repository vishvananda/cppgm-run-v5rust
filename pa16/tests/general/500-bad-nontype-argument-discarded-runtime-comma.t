// VALIDATION: compile-fail
// The discarded left operand of a comma expression in a non-type template
// argument must still be a constant expression.

int runtime_value();

template<int>
struct marker
{
};

marker<(runtime_value(), 1)> invalid;

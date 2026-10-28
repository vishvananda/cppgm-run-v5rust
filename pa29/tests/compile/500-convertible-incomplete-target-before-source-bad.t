// The target's initialization precondition precedes source operand typing.
template<class T> struct From { static_assert(sizeof(T) == 0, "source completed"); };
struct To;
static_assert(__is_convertible(From<int>, To), "conversion");

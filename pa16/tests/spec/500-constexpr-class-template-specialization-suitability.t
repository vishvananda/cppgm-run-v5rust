template<class T> struct box { constexpr box() {} T value; };
static_assert(sizeof(box<int *>) > 0, "class specialization is complete");

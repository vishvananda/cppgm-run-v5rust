struct payload { unsigned long low, high; };
struct box { _Atomic(payload) value; };
static_assert(alignof(payload) == 8, "");
static_assert(alignof(box) == 16, "");

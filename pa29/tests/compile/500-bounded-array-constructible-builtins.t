static_assert(__is_constructible(int[2]), "");
static_assert(__is_nothrow_constructible(int[2]), "");
static_assert(!__is_constructible(const int[2], const int(&&)[2]), "");

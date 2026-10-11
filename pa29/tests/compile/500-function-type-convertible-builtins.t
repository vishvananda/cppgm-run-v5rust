static_assert(__is_convertible(int(), int(*)()), "");
static_assert(__is_convertible(int(), int(&)()), "");
static_assert(__is_convertible(int(), int(&&)()), "");

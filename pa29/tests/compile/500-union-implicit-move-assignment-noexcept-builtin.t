union U { int value; };
static_assert(__is_nothrow_assignable(U&, U&&), "");

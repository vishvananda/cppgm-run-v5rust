// VALIDATION: compile-pass

static_assert(__is_same(__remove_cv(const int[2]), int[2]), "");

int main() {}

// HHC-152
static_assert('\0' == 0, "");
static_assert(L'\0' == 0, "");

int main() { return 0; }

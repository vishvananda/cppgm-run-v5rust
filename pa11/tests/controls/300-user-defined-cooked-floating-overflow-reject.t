// N3485 2.14.8p4: cooked selection demands long double range; expect reject
int operator ""_tag(long double) { return 0; }
int operator ""_tag(const char *) { return 1; }
int main() { return 1e999999999_tag; }

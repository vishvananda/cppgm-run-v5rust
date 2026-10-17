// N3485 2.14.8p3: cooked selection demands unsigned long long range; expect reject
int operator ""_tag(unsigned long long) { return 0; }
int operator ""_tag(const char *) { return 1; }
int main() { return 18446744073709551616_tag; }

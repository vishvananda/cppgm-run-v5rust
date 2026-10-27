constexpr unsigned long operator ""s(const char *, unsigned long length) { return length; }
constexpr unsigned long first = "abc"s;
constexpr unsigned long joined = "a"s "bc";
static_assert(first == 3 && joined == 3, "reserved string suffix");
int main() { return first + joined - 6; }

// expect exit 0
const long &unused_return() { return {3}; }
const short &unused_short_return() { return {3}; }
int main() {
    const long &a = static_cast<const long &>(1);
    const long &b{3};
    const long &c = 4;
    const long &d{};
    const double &e{3};
    return a != 1 || b != 3 || c != 4 || d != 0 || e != 3;
}

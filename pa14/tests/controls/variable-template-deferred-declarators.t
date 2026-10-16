// expect exit 0
template<class T> void unused() {
    int local<T>;
}
template<class T> struct Unused {
    int member<T>;
};
int main() { return 0; }

// expect exit 0
int destroyed = 0;
struct Base {
    Base() {}
protected:
    ~Base() { ++destroyed; }
};
struct Derived : Base {};
int main() {
    { Derived value; }
    return destroyed != 1;
}

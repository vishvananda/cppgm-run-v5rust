// N3485 11p4 + PA17 variable templates: a private static data member template
// named outside its class is inaccessible; expect reject
class C { template <class T> static constexpr int v = 1; };
int main() { return C::v<int>; }

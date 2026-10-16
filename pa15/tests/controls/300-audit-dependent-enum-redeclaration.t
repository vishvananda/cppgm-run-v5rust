// expect exit 0
template<class T> struct A { enum E : T; };
template<> enum A<int>::E : int { x = -1, y };
template<class Renamed> enum A<Renamed>::E : Renamed { a = 0, b = 1 };
int main() { return A<int>::y + A<unsigned char>::b - 1; }

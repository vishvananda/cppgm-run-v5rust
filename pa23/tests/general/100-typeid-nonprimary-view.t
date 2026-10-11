namespace std {
class type_info { public: bool operator==(const type_info&) const; bool operator!=(const type_info&) const; };
}

class YA { public: virtual int f() { return 1; } };
class YB { public: virtual int g() { return 2; } };
class YD : public YA, public YB {};
int main() { YD d; YB &b = d; return typeid(b) == typeid(YD) ? 1 : 0; }

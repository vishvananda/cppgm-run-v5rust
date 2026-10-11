class YB { public: int f() { return 1; } };
class YD : public YB { public: virtual int g() { return 2; } };
int main() { YD d; return d.g(); }

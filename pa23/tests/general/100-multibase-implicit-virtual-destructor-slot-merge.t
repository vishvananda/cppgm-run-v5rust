class YA { public: virtual int g() { return 0; } virtual ~YA() {} };
class YB { public: virtual ~YB() {} };
class YC : public YA, public YB {};
int main() { YC c; return 0; }

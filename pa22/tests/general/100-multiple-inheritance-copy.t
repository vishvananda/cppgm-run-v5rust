class YA { public: int a; YA(): a(1) {} };
class YB { public: int b; YB(): b(2) {} };
class YC : public YA, public YB { };
int main() { YC c; YC d = c; d.a = 4; return c.a + c.b + d.a + d.b; }

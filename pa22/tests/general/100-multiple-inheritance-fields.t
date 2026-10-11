class YA { public: int a; };
class YB { public: int b; };
class YC : public YA, public YB { };
int main() { YC c; c.a = 2; c.b = 3; return c.a + c.b; }

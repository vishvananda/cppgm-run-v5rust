class YA { public: int a; int fa() { return a; } };
class YB { public: int b; int gb() { return b; } };
class YC : public YA, public YB { };
int main() { YC c; c.a = 2; c.b = 3; return c.fa() + c.gb(); }

class YA { public: int x; };
class YB { public: int x; };
class YC : public YA, public YB { };
int main() { YC c; return c.x; }

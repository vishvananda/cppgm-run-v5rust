class B0 { public: virtual ~B0() {} };
class B1 : public B0 { public: ~B1() override {} };
class B2 : public B0 { public: ~B2() override {} };
class D : public B1, public B2 { public: ~D() override {} };
int main() { D d; return 0; }

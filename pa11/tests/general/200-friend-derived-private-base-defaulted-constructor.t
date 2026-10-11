class B { friend class D; B() {} };
class D : B { public: D() = default; };
int main() { D d; }

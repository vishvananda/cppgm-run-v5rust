// Virtual bases prevent an implicit constructor's constant evaluation,
// even when their only storage is a shared primary vptr; expect exit 0
struct Root { virtual int value() { return 3; } };
struct Child : virtual Root {};
struct Indirect : Child {};
Child child;
Indirect indirect;
Child children[2];
int main()
{
    Root* a = &child;
    Root* b = &indirect;
    Root* c = &children[1];
    return a->value() + b->value() + c->value() == 9 ? 0 : 1;
}

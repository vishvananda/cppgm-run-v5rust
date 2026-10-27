struct Base { int value; };
struct Derived : Base {};
Base* up(Derived* pointer) { return pointer; }
Derived* down(Base* pointer) { return static_cast<Derived*>(pointer); }
int main() {
    Derived object;
    object.value = 7;
    Base* first = &object;
    Base* second = &object;
    Base& reference = object;
    return first->value + second->value + reference.value + down(up(&object))->value - 28;
}

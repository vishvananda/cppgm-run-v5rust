// expect exit 0
int calls;
struct S {
    int value;
    static int get() { return 7; }
};
struct Holder { S member; };
S object;
S* source() { ++calls; return &object; }
int effect() { ++calls; return 0; }
int main() {
    S local;
    S* pointer = &local;
    S* volatile volatile_pointer = &local;
    Holder holder;
    S array[2];
    volatile S volatile_object;
    int value = local.get() + pointer->get() + holder.member.get() + array[1].get();
    value += volatile_pointer->get() + volatile_object.get();
    value += source()->get() + (effect(), local).get();
    return value == 56 && calls == 2 ? 0 : 1;
}

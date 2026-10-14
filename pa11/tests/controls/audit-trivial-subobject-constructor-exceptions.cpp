// expect exit 0
struct Leaf { Leaf() noexcept(false) = default; };
struct Implicit { Leaf leaf; int value = 1; };
struct Array { Leaf leaves[2]; int value = 2; };
struct Defaulted { Leaf leaf; int value = 3; Defaulted() = default; };
int main() {
    Implicit implicit;
    Array array;
    Defaulted defaulted;
    return implicit.value + array.value + defaulted.value == 6 ? 0 : 1;
}

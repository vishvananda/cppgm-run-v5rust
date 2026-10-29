// A pointer first zeroed through a bulk operation remains live across a cold
// call. Exercise the frameless return and the framed allocation path.
void* operator new(__SIZE_TYPE__, void* place) noexcept { return place; }
struct Ptr { long* p; };
struct A {
    Ptr owner;
    A() : owner() {}
    A(const A& other) : owner() {
        if (other.owner.p) owner.p = new long(*other.owner.p);
    }
};
void copy(A* destination, const A* source) { new(destination) A(*source); }
int main() {
    A empty;
    A result;
    copy(&result, &empty);
    if (result.owner.p) return 1;
    long value = 73;
    empty.owner.p = &value;
    copy(&result, &empty);
    if (!result.owner.p || *result.owner.p != value) return 2;
    delete result.owner.p;
}

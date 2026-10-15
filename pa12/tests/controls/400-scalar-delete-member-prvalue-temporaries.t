// expect exit 0
// A trivial containing prvalue still opens its argument-temporary region.
int destroyed;
struct Temporary { ~Temporary() { ++destroyed; } };
struct Object { ~Object() noexcept(false) { ++destroyed; } };
struct Pointer { Object* pointer; };
Pointer make(const Temporary&) { return Pointer{new Object}; }
void release() { delete make(Temporary()).pointer; }
int main() { release(); return destroyed == 2 ? 0 : 1; }

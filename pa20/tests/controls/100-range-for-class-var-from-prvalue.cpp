// N3485 6.5.4p1: `for (Val v : seq)` copy-initializes v from the class prvalue
// *__begin returned by a user iterator; expect exit 0
struct Val { int n; Val(int x) : n(x) {} Val(const Val& o) : n(o.n) {} ~Val() {} };
struct It { const int* p; Val operator*() const { return Val(*p); } It& operator++() { ++p; return *this; } bool operator!=(const It& o) const { return p != o.p; } };
struct Seq { int d[3]; It begin() const { It i = {d}; return i; } It end() const { It i = {d + 3}; return i; } };
int main() { const Seq seq = {{4, 5, 6}}; int s = 0; for (Val v : seq) s += v.n; return s == 15 ? 0 : 1; }

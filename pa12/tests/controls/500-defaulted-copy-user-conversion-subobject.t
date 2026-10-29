// expect exit 0
// Subobject selection inherits its conversion policy. A const source cannot
// bind the nonconst copy parameter, but can select Member(int) via operator int.
struct Member { Member(Member&); Member(int); operator int() const; };
struct Owner { Member member; Owner(const Owner&) = default; };
void accepts(Owner);
inline void check(const Owner& value) { accepts(value); }
int main() { return 0; }

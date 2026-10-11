// An unproved member's complete layout must retain runtime projection.
struct Virtual { int value; virtual ~Virtual() {} };
struct Primary { int primary; virtual ~Primary() {} };
struct Base : Primary, virtual Virtual { int base; };
struct Object : Base { int object; };
struct Holder { long prefix; Object member; };
Holder holder;
Virtual &reference = holder.member;
Virtual *pointer = &holder.member;
Virtual *explicit_pointer = static_cast<Virtual *>(&holder.member);
Base &base_reference = holder.member;
Virtual &alias_reference = base_reference;
int main() {
  reference.value = 7;
  Virtual *expected = static_cast<Virtual *>(&holder.member);
  return holder.member.value != 7 || pointer != expected ||
    explicit_pointer != expected || &alias_reference != expected;
}

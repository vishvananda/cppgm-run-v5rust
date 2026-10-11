// Preserve the binding identity and the member/array origin.
struct Left { int left; };
struct Right { int right; };
struct Object : Left, Right { int value; };
struct Holder { long prefix; Object member; };
struct Further : Object { int further; };
Holder holder;
Object objects[3];
Further further;
Right &member_reference = holder.member;
Right *member_pointer = &holder.member;
Right &array_reference = objects[1];
Right &nested_reference = further;
Right &alias = array_reference;
int main() {
  member_reference.right = 3;
  array_reference.right = 5;
  nested_reference.right = 7;
  return holder.member.right != 3 || objects[1].right != 5 ||
    further.right != 7 || objects[0].right != 0 || objects[2].right != 0 ||
    member_pointer != static_cast<Right *>(&holder.member) ||
    &alias != static_cast<Right *>(&objects[1]);
}

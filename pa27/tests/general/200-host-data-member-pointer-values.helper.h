// Data member pointer values cross object boundaries in the Itanium C++ ABI
// representation (2.3): the byte offset, -1 for null.
namespace member_values {
struct S { int a; int b; int c; };
struct B1 { long x; };
struct B2 { int y; int z; };
struct D : B1, B2 { int w; };

int S::* host_member(int which);
bool host_is_null(int S::* pointer);
int host_read(const S& object, int S::* pointer);
int D::* host_widen(int B2::* pointer);
int host_check_student();

int S::* student_member(int which);
bool student_is_null(int S::* pointer);
int student_read(const S& object, int S::* pointer);
int D::* student_widen(int B2::* pointer);
}

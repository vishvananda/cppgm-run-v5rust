// Static data holding data member pointers uses the Itanium representation
// (2.3) in both directions: explicit values are byte offsets, and null and
// zero-initialized members (aggregate tails, unset static objects) are -1.
namespace member_data {
struct S { int a; int b; int c; };
typedef int S::* Member;
struct Entry { Member member; int tag; };
struct Table { int count; Entry entries[3]; Member extra; };

extern Member host_null, host_b;
extern Entry host_unset;
extern Entry host_entries[4];
extern Table host_table;
int host_check_student();

extern Member student_null, student_c;
extern Entry student_unset;
extern Entry student_entries[4];
extern Table student_table;
}

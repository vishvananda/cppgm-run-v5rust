// N3485 9.2/13: every member type of class T shall have a name different from
// T; must be rejected
struct A { struct A { }; typedef int A2; };
struct B { typedef int B; };

#pragma pack(push,1)
struct A { void f() { struct Local {}; } } __attribute__((packed));
#pragma pack(push,4)
struct B {};
#pragma pack(pop)
struct C {};
#pragma pack(pop)
struct D {};

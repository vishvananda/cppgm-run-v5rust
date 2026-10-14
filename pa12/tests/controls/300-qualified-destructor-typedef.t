// N3485 3.4.3p6, 12.4p13: in p->N::~N() with N a typedef-name for the class,
// ~N is looked up like N and names the destructor; expect exit 0
int count;
struct B { ~B() { ++count; } };
typedef B B_alias;
int main() { B* q = new B; q->B_alias::~B_alias(); ::operator delete(q); return count == 1 ? 0 : 1; }

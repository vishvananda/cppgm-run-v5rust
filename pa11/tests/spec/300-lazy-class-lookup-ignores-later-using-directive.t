// N3485 focus: 3.4.1 [basic.lookup.unqual] lookup at the declaration point
typedef int T;
struct X { T value; };
namespace N { enum T { item }; }
using namespace N;
void expect(int *);
int main() { X x; expect(&x.value); }

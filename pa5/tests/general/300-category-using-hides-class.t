namespace N { struct C { int word; }; int C; }
struct C { typedef int word; };
using N::C;

struct base { typedef int word; };
template<class T> struct P : T {};
namespace Alias { using imported = P<base>; }
using Alias::imported;

void syntax() {
  C::word *local_class;
  imported::word *imported_category;
}

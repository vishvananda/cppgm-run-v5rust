namespace Imported {
template<class T> struct begin {};
template<class T> struct end {};
}

using namespace Imported;

struct Span
{
  int begin;
  int end;
};

bool outside(Span span, Span root)
{
  return span.begin < root.begin || span.end > root.end;
}

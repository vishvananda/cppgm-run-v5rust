template<class T> struct P { typedef int word; };
template<> struct P<int> { int word; };
int range;

void syntax() {
  if (P<int>::word *p = 0) { P<int>::word *q; }
  while (P<int>::word *p = 0) { break; }
  for (P<int>::word *p = 0; p; ++p) { P<int>::word *q; }
  for (P<int>::word p : range) { P<int>::word *q; }
  switch (P<int>::word *p = 0) { case 0: break; }
}

// A base initialized from a prvalue of its own class writes only the bytes
// the base owns: the virtual base Tail, placed in Padded's tail padding and
// constructed first, keeps its value.
struct Tail { char tail; Tail() : tail(9) {} };
struct Padded { Padded(int x, char y) : i(x), c(y) {} int i; char c; };
Padded make_padded() { return Padded(1, 2); }
struct Holder : virtual Tail, Padded { Holder() : Padded(make_padded()) {} };

int main()
{
  Holder holder;
  if (reinterpret_cast<char*>(static_cast<Tail*>(&holder)) -
      reinterpret_cast<char*>(&holder) != 13) return 1;
  if (holder.tail != 9 || holder.i != 1 || holder.c != 2) return 2;
  return 0;
}

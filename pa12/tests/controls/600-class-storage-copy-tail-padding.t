// expect exit 0
// Class storage transfers exclude tail padding; object-value parameters and
// results keep their full ABI width, and arrays keep their element stride.
struct Data {
  short value;
  char tag;
  Data(short n = 3) : value(n), tag(7) {}
};
struct Holder { Data data; char last; };
Data by_value(Data value) { return value; }
Data by_ref(const Data& value) { return value; }
Holder copy_holder(const Holder& value) { return value; }
void assign(Data& left, const Data& right) { left = right; }
struct Array { Data data[2]; };
int main() {
  Data source(9);
  Data copy(source);
  Data moved(static_cast<Data&&>(source));
  Data returned = by_value(copy);
  Data referred = by_ref(moved);
  Holder holder;
  holder.last = 5;
  Holder other = copy_holder(holder);
  assign(returned, referred);
  Array first;
  first.data[1].value = 11;
  Array second(first);
  return returned.value != 9 || returned.tag != 7 || other.last != 5
      || second.data[0].value != 3 || second.data[1].value != 11;
}

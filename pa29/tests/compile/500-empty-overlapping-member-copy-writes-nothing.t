// An empty class carries no value, so copying one has nothing to write.  A
// member declared [[no_unique_address]] shares its address with the member it
// overlaps, so a store of the empty class's padding byte would land on that
// neighbour's storage.
struct Empty {};

Empty make_empty() { return Empty(); }

struct Table
{
  void* head;
  [[__no_unique_address__]] Empty tag;
  Table() : head(0), tag(make_empty()) {}
};

int main()
{
  Table table;
  return table.head == 0 ? 0 : 1;
}

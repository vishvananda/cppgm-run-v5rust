// lowir.md storage=readonly: a statically initialized const pointer to member,
// data or function, states storage=readonly, as a const array of them does;
// a non-const one does not.
struct Record
{
  long key;
  long value;
  long sum() const { return key + value; }
};

long Record::* const value_member = &Record::value;
long (Record::* const sum_member)() const = &Record::sum;
long Record::* const members[2] = {&Record::key, &Record::value};
long Record::* changeable_member = &Record::key;

int main()
{
  Record record = {1, 2};
  return record.*value_member + (record.*sum_member)() + record.*members[0] +
    record.*changeable_member == 2 + 3 + 1 + 1 ? 0 : 1;
}

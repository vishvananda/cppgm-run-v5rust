// expect reject
// N3485 14.2p4: check an unused template definition, through a cv pointer.
template<class T>
int invoke(const T *const object)
{
  return object->apply<int>();
}

int main() { return 0; }

struct item {
  item* child;
};

void take_pointer(item**);
void take_reference(item&);
bool test_reference(item&);

void use(item** value, item holder) {
  take_pointer(&(*value));
  take_reference(*holder.child);
  if (test_reference(*(*value))) {}
}

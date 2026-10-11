typedef void function_type(int);
extern function_type* target;

void call_target(int value) {
  (target)(value);
}

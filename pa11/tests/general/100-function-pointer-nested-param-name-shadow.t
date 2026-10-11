struct error_code {
  int value;
};

struct reactor_op {
  error_code ec_;
};

int set_internal_non_blocking(int, unsigned char &, bool, error_code &ec)
{
  return ec.value;
}

void callback(reactor_op *, bool)
{
}

int do_start_op(int descriptor,
                unsigned char &state,
                reactor_op *op,
                void (*on_immediate)(reactor_op *op, bool))
{
  (void)on_immediate;
  return set_internal_non_blocking(descriptor, state, true, op->ec_);
}

int main()
{
  unsigned char state = 0;
  reactor_op op;
  op.ec_.value = 7;
  return do_start_op(1, state, &op, callback) == 7 ? 0 : 1;
}

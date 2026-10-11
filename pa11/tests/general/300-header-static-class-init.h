extern int counter;

struct HeaderInit {
  HeaderInit() { counter = 7; }
  ~HeaderInit() { counter = counter + 1; }
};

static HeaderInit header_init;

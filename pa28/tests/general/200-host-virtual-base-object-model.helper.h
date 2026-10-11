// The Itanium virtual-base object model across a g++ object: vbase-offset
// rows (first virtual base nearest the address point), ordinary member
// functions with no hidden virtual-base arguments, projection to an
// indirect virtual base through its own row, and a complete constructor
// built here driving g++ base-object constructors and destructors through
// the VTT, virtual bases first and depth-first.
struct RowA { int a; };
struct RowB { int b; };
struct RowC { int c; };
struct Rows : virtual RowA, virtual RowB, virtual RowC { int r; };

struct Flags { int flags; void set(int v) { flags = v; } };
struct Marker : virtual Flags { int pad; void mark(int v); };

struct IOS { int value; };
struct BasicIOS : virtual IOS { int pad; };
struct OStream : virtual BasicIOS { int o; };
struct Other : virtual IOS { int other[5]; };
struct Weird : Other, OStream { int w; };

extern int order[8];
extern int order_count;
struct Root { int root; Root(); ~Root(); };
struct Middle : virtual Root { int middle; Middle(); ~Middle(); virtual int which(); };
struct Leaf : virtual Middle { int leaf; Leaf(); ~Leaf(); virtual int which(); };

int host_read_rows(const Rows& rows);
void host_fill_rows(Rows& rows);
Rows* host_make_rows();
int host_read_value(OStream* out);
Weird* host_make_weird();
void host_mark(Marker& marker, int value);
int host_which(Middle& middle);

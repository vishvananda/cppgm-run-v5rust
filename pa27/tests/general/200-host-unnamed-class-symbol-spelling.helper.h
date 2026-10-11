// A class a typedef names for linkage purposes is that name
// (_ZNK14NamedByTypedef3getEv); an unnamed member class of a named class is
// Ut_ in its class (_Z12unnamed_sizeIN6HolderUt_EEiPT_).
typedef struct
{
  int y;
  int get() const;
} NamedByTypedef;

struct Holder
{
  struct
  {
    int a;
    int b;
  } pair;
};

template<class T> int unnamed_size(T *);

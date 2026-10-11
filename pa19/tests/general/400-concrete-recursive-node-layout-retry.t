// A concrete node specialization can be structurally completed while its
// recursively referenced value type still lacks layout.  Once the value type
// completes, a later node-allocation demand must rematerialize that same
// specialization instead of treating its earlier no-layout shape as final.

using b = decltype(0);
namespace boost {
namespace container {
namespace bi = boost;
template <class> class c;
} // namespace container
template <typename d> struct aa {
  typedef d ab;
};
char e;
struct f;
template <int h> struct i {
  typedef int ab;

private:
};
namespace container {
using ::boost ::f;
using ::boost ::i;
template <class j> class k {
public:
  typedef j l;
  template <class d> struct m {
    typedef k<d> n;
  };
};
} // namespace container
struct ac {
  template <typename o> static char p(int, typename o ::template m<int> *);
  static const bool g = 1 == sizeof(p<container::k<int>>(0, 0));
};
struct ae {
  template <typename o> static char m_fn4(int, typename o ::template m<int> *);
  static const bool g = 1 == sizeof(m_fn4<boost::container::k<int>>(0, 0));
};
struct r {
  static const unsigned int m = ac::g;
  static const unsigned int s = ae::g;
  static const unsigned int af = ac::g + ac::g * s;
};
template <typename d, typename q, unsigned int t> struct ag;
template <typename ad, typename q> struct ag<ad, q, 2> {
  typedef typename ad ::template m<q>::n ab;
};
template <typename ad, typename q> struct ah : public ag<ad, q, r::af> {};
template <typename d> struct ai;
template <typename j> struct ai<j *> {
  typedef j u;
  template <class q> struct aj {
    typedef q *ab;
  };
};
namespace container {
template <typename v> struct w {
  struct x {
    typedef v ak;
  };
  typedef typename ::boost ::aa<x>::ab ::ak ab;
};
template <typename y> struct z {
  typedef typename boost ::container ::w<typename y::l *>::ab ak;
  template <typename j> using al = typename boost ::ah<y, j>::b;
  template <typename j> using am = z<al<j>>;
  template <class j> struct an {
    typedef typename boost ::ah<y, j>::ab ab;
  };
};
template <class j> struct ao;
template <class j> struct ao {
  typedef k<j> ab;
};
} // namespace container
enum ap {
  aq,
};
template <class ar> struct as {
  typedef ar at;
};
template <b... a> struct av;
template <b aw, typename ax = av<>> struct ay;
template <b aw, b... au>
struct ay<aw, av<au...>> : ay<aw - 1, av<au..., sizeof...(au)>> {};
template <b... au> struct ay<0, av<au...>> {
  typedef av<au...> ab;
};
template <class... az> struct ba;
template <class j> struct bb;
template <b c, typename ax> struct bd;
template <b bc, typename be, typename... bf> struct bd<bc, ba<be, bf...>> {
  typedef typename bd<bc - 1, ba<bf...>>::ab ab;
};
template <typename be, typename... bf> struct bd<0, ba<be, bf...>> {
  typedef be ab;
};
template <class bg> struct bh;
template <class... az> struct bh<ba<az...>> {
  static const b g = sizeof...(az);
};
template <class bg, class a> struct cf;
template <class bg, b... bj> struct cf<bg, av<bj...>> {
  static const b bk = bh<bg>::g - 1;
  typedef ba<typename bd<bk - bj, bg>::ab...> ab;
};
template <class... az> struct bb<ba<az...>> {
  typedef typename cf<ba<az...>, typename ay<sizeof...(az)>::ab>::ab ab;
};
template <class bg> struct bl;
template <class bm> struct bl<ba<bm>> {
  typedef bm ab;
};
template <class bm, class... bn> struct bl<ba<bm, bn...>> {
  typedef typename bm ::template bo<typename bl<ba<bn...>>::ab> ab;
};
template <class da, class... Options> struct bp {
  typedef typename bl<typename bb<ba<int, Options...>>::ab>::ab ab;
};
struct bq {
  template <class br> struct bo : br {};
};
struct bs {
  template <class br> struct bo : br {};
};
template <class bt> struct bu {
  template <class br> struct bo {
    typedef bt bv;
  };
};
template <class ar> struct bw {
  template <class br> struct bo {
    typedef ar bw;
  };
};
template <ap bx> struct by {
  template <class br> struct bo : br {
    static const ap by = bx;
  };
};
enum bz {
  ca,
};
template <class cb, ap cc, int cd> struct ce {
  static const ap by = cc;
  typedef cb cg;
  static const unsigned int b = cd;
};
template <class cb, ap cc, int cd> class ch {
public:
  typedef ce<cb, cc, cd> ci;

public:
};
template <class... Options> struct cj {
  typedef typename bp<int, Options...>::ab ck;
  typedef ch<as<typename ck::bw>, ck::by, ca> ab;
};
template <class j, class cl> struct bhtraits_base {
public:
  typedef typename ai<cl>::template aj<j>::ab ak;
};
template <class j, class cb>
struct bhtraits : public bhtraits_base<j, typename cb ::at> {};
struct cm {
  static const bool g = false;
};
template <class j, class bt> struct cn {
  typedef typename bt ::ci co;
  typedef bhtraits<j, typename co ::cg> ab;
};
struct dn {
  static const b g = sizeof(0);
};
struct cp {
  static const bool g = dn::g > sizeof 0 * 2;
};
struct cq {
  static const b g = sizeof 0;
};
struct cr {
  static const bool g = cq::g > sizeof(e) * 2;
};
template <class cs, class j, bool = cm::g> struct ct;
template <class j, class bt, bool = cp::g> struct cu;
template <class cs, class j, bool = cr::g> struct cv;
template <class j, class bt> struct cu<j, bt, false> : cn<j, bt> {};
template <class cs, class j> struct ct<cs, j, false> {
  typedef cs ab;
};
template <class bt, class j> struct cv<bt, j, true> : cu<j, bt> {};
template <class j, class cs> struct cw : cv<typename ct<cs, j>::ab, j> {};
template <class cx> class cy {
public:
  typedef typename ai<typename cx::ak>::u l;

private:
};
template <class j, class... Options> struct cz {
  typedef typename bp<int, Options...>::ab ck;
  typedef cy<typename cw<j, typename ck::bv>::ab> ab;
};
namespace container {
template <class j> struct base_node {
  typename i<sizeof(j)>::ab a;

public:
  template <class db, class... c> explicit base_node(db &p1) noexcept;

private:
};
template <class y, class dd> struct de {
  typedef typename dd ::l Node;
  typedef typename z<y>::template an<Node>::ab df;
  typedef typename z<df>::ak c;

private:
public:
  template <class... dc> void m_fn5(dc &&...p1) {
    df &dh = this->m_fn1();
    ::new Node(dh, p1...);
  }
  df &m_fn1();
};
template <class ar> struct dj {
  typedef typename bi ::cj<bi ::bw<ar>, bi ::by<bi ::aq>>::ab ab;
};
template <class y> struct dk {
  typedef typename bi::cz<
      base_node<typename y::l>,
      bi::bu<typename dj<typename boost::ai<
          typename boost::container::z<y>::ak>::template aj<void>::ab>::ab>,
      bi::bq, bi::bs>::ab ab;
};
template <class j>
class c
    : protected de<typename ao<j>::ab, typename dk<typename ao<j>::ab>::ab> {
public:
  inline void m_fn2() { this->m_fn5(); }
};
} // namespace container
} // namespace boost
class recursive_slist {
public:
  boost ::container ::c<recursive_slist> dm;
};
int main() {
  recursive_slist g;
  g.dm.m_fn2();
}

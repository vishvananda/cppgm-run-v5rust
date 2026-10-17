// N3485 7.1.6.3p2, 11.3: a qualified friend cannot introduce a new class.
// expect reject
namespace Existing {}
class Owner {
  friend class Existing::Missing;
};

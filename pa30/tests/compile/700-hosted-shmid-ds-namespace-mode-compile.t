#include <sys/shm.h>

namespace boost {
namespace interprocess {

enum mode_t {
  read_only,
  read_write
};

struct mapping_handle_t {
  bool is_xsi;
  int handle;
};

struct mappable {
  mapping_handle_t get_mapping_handle() const;
};

template<class MemoryMappable>
struct mapped_region {
  mapped_region(const MemoryMappable & mapping, mode_t mode)
  {
    (void)mode;
    mapping_handle_t map_hnd = mapping.get_mapping_handle();
    if(map_hnd.is_xsi) {
      ::shmid_ds xsi_ds;
      int ret = ::shmctl(map_hnd.handle, IPC_STAT, &xsi_ds);
      (void)ret;
      unsigned long size = static_cast<unsigned long>(xsi_ds.shm_segsz);
      (void)size;
    }
  }
};

}
}

int main()
{
  boost::interprocess::mappable mapping;
  boost::interprocess::mapped_region<boost::interprocess::mappable> region(
      mapping,
      boost::interprocess::read_only);
  (void)region;
  return 0;
}

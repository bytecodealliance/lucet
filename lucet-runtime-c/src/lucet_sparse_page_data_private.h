
#ifndef LUCET_SPARSE_PAGE_DATA_PRIVATE_H
#define LUCET_SPARSE_PAGE_DATA_PRIVATE_H

struct lucet_sparse_page_data {
    uint64_t num_pages;
    uint8_t *pages[];
};

#endif // LUCET_SPARSE_PAGE_DATA_PRIVATE_H

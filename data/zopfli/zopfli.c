#include "zopfli.h"

#include <stddef.h>
#include <stdlib.h>
#include <stdio.h>
#include <assert.h>
#include <x86_64-linux-gnu/bits/types/FILE.h>
#include <limits.h>
#include <string.h>
#include <math.h>

/*
Gets the symbol for the given length, cfr. the DEFLATE spec.
Returns the symbol in the range [257-285] (inclusive)
*/
static int ZopfliGetLengthSymbol(int l)
{
    static const int table[259] = {
        0, 0, 0, 257, 258, 259, 260, 261, 262, 263, 264,
        265, 265, 266, 266, 267, 267, 268, 268,
        269, 269, 269, 269, 270, 270, 270, 270,
        271, 271, 271, 271, 272, 272, 272, 272,
        273, 273, 273, 273, 273, 273, 273, 273,
        274, 274, 274, 274, 274, 274, 274, 274,
        275, 275, 275, 275, 275, 275, 275, 275,
        276, 276, 276, 276, 276, 276, 276, 276,
        277, 277, 277, 277, 277, 277, 277, 277,
        277, 277, 277, 277, 277, 277, 277, 277,
        278, 278, 278, 278, 278, 278, 278, 278,
        278, 278, 278, 278, 278, 278, 278, 278,
        279, 279, 279, 279, 279, 279, 279, 279,
        279, 279, 279, 279, 279, 279, 279, 279,
        280, 280, 280, 280, 280, 280, 280, 280,
        280, 280, 280, 280, 280, 280, 280, 280,
        281, 281, 281, 281, 281, 281, 281, 281,
        281, 281, 281, 281, 281, 281, 281, 281,
        281, 281, 281, 281, 281, 281, 281, 281,
        281, 281, 281, 281, 281, 281, 281, 281,
        282, 282, 282, 282, 282, 282, 282, 282,
        282, 282, 282, 282, 282, 282, 282, 282,
        282, 282, 282, 282, 282, 282, 282, 282,
        282, 282, 282, 282, 282, 282, 282, 282,
        283, 283, 283, 283, 283, 283, 283, 283,
        283, 283, 283, 283, 283, 283, 283, 283,
        283, 283, 283, 283, 283, 283, 283, 283,
        283, 283, 283, 283, 283, 283, 283, 283,
        284, 284, 284, 284, 284, 284, 284, 284,
        284, 284, 284, 284, 284, 284, 284, 284,
        284, 284, 284, 284, 284, 284, 284, 284,
        284, 284, 284, 284, 284, 284, 284, 285};
    return table[l];
}

/* Gets the symbol for the given dist, cfr. the DEFLATE spec. */
static int ZopfliGetDistSymbol(int dist)
{
    if (dist < 5)
    {
        return dist - 1;
    }
    else
    {
        int l = (31 ^ __builtin_clz(dist - 1)); /* log2(dist - 1) */
        int r = ((dist - 1) >> (l - 1)) & 1;
        return l * 2 + r;
    }
}

static size_t AbsDiff(size_t x, size_t y)
{
    if (x > y)
        return x - y;
    else
        return y - x;
}

/*
Changes the population counts in a way that the consequent Huffman tree
compression, especially its rle-part, will be more likely to compress this data
more efficiently. length contains the size of the histogram.
*/
void OptimizeHuffmanForRle(int length, size_t *counts)
{
    int i, k, stride;
    size_t symbol, sum, limit;
    int *good_for_rle;

    /* 1) We don't want to touch the trailing zeros. We may break the
    rules of the format by adding more data in the distance codes. */
    for (; length >= 0; --length)
    {
        if (length == 0)
        {
            return;
        }
        if (counts[length - 1] != 0)
        {
            /* Now counts[0..length - 1] does not have trailing zeros. */
            break;
        }
    }
    /* 2) Let's mark all population counts that already can be encoded
    with an rle code.*/
    good_for_rle = (int *)malloc((unsigned)length * sizeof(int));
    for (i = 0; i < length; ++i)
        good_for_rle[i] = 0;

    /* Let's not spoil any of the existing good rle codes.
    Mark any seq of 0's that is longer than 5 as a good_for_rle.
    Mark any seq of non-0's that is longer than 7 as a good_for_rle.*/
    symbol = counts[0];
    stride = 0;
    for (i = 0; i < length + 1; ++i)
    {
        if (i == length || counts[i] != symbol)
        {
            if ((symbol == 0 && stride >= 5) || (symbol != 0 && stride >= 7))
            {
                for (k = 0; k < stride; ++k)
                {
                    good_for_rle[i - k - 1] = 1;
                }
            }
            stride = 1;
            if (i != length)
            {
                symbol = counts[i];
            }
        }
        else
        {
            ++stride;
        }
    }

    /* 3) Let's replace those population counts that lead to more rle codes. */
    stride = 0;
    limit = counts[0];
    sum = 0;
    for (i = 0; i < length + 1; ++i)
    {
        if (i == length || good_for_rle[i]
            /* Heuristic for selecting the stride ranges to collapse. */
            || AbsDiff(counts[i], limit) >= 4)
        {
            if (stride >= 4 || (stride >= 3 && sum == 0))
            {
                /* The stride must end, collapse what we have, if we have enough (4). */
                int count = (sum + stride / 2) / stride;
                if (count < 1)
                    count = 1;
                if (sum == 0)
                {
                    /* Don't make an all zeros stride to be upgraded to ones. */
                    count = 0;
                }
                for (k = 0; k < stride; ++k)
                {
                    /* We don't want to change value at counts[i],
                    that is already belonging to the next stride. Thus - 1. */
                    counts[i - k - 1] = count;
                }
            }
            stride = 0;
            sum = 0;
            if (i < length - 3)
            {
                /* All interesting strides have a count of at least 4,
                at least when non-zeros. */
                limit = (counts[i] + counts[i + 1] +
                         counts[i + 2] + counts[i + 3] + 2) /
                        4;
            }
            else if (i < length)
            {
                limit = counts[i];
            }
            else
            {
                limit = 0;
            }
        }
        ++stride;
        if (i != length)
        {
            sum += counts[i];
        }
    }

    free(good_for_rle);
}

/* Gets the amount of extra bits for the given distance symbol. */
static int ZopfliGetDistSymbolExtraBits(int s)
{
    static const int table[30] = {
        0, 0, 0, 0, 1, 1, 2, 2, 3, 3, 4, 4, 5, 5, 6, 6, 7, 7, 8, 8,
        9, 9, 10, 10, 11, 11, 12, 12, 13, 13};
    return table[s];
}

/* Gets the amount of extra bits for the given length symbol. */
static int ZopfliGetLengthSymbolExtraBits(int s)
{
    static const int table[29] = {
        0, 0, 0, 0, 0, 0, 0, 0, 1, 1, 1, 1, 2, 2, 2, 2,
        3, 3, 3, 3, 4, 4, 4, 4, 5, 5, 5, 5, 0};
    return table[s - 257];
}

/*
Same as CalculateBlockSymbolSize, but for block size smaller than histogram
size.
*/
static size_t CalculateBlockSymbolSizeSmall(const unsigned *ll_lengths,
                                            const unsigned *d_lengths,
                                            const ZopfliLZ77Store *lz77,
                                            size_t lstart, size_t lend)
{
    size_t result = 0;
    size_t i;
    for (i = lstart; i < lend; i++)
    {
        assert(i < lz77->size);
        assert(lz77->litlens[i] < 259);
        if (lz77->dists[i] == 0)
        {
            result += ll_lengths[lz77->litlens[i]];
        }
        else
        {
            int ll_symbol = ZopfliGetLengthSymbol(lz77->litlens[i]);
            int d_symbol = ZopfliGetDistSymbol(lz77->dists[i]);
            result += ll_lengths[ll_symbol];
            result += d_lengths[d_symbol];
            result += ZopfliGetLengthSymbolExtraBits(ll_symbol);
            result += ZopfliGetDistSymbolExtraBits(d_symbol);
        }
    }
    result += ll_lengths[256]; /*end symbol*/
    return result;
}

/*
Same as CalculateBlockSymbolSize, but with the histogram provided by the caller.
*/
static size_t CalculateBlockSymbolSizeGivenCounts(const size_t *ll_counts,
                                                  const size_t *d_counts,
                                                  const unsigned *ll_lengths,
                                                  const unsigned *d_lengths,
                                                  const ZopfliLZ77Store *lz77,
                                                  size_t lstart, size_t lend)
{
    size_t result = 0;
    size_t i;
    if (lstart + ZOPFLI_NUM_LL * 3 > lend)
    {
        return CalculateBlockSymbolSizeSmall(
            ll_lengths, d_lengths, lz77, lstart, lend);
    }
    else
    {
        for (i = 0; i < 256; i++)
        {
            result += ll_lengths[i] * ll_counts[i];
        }
        for (i = 257; i < 286; i++)
        {
            result += ll_lengths[i] * ll_counts[i];
            result += ZopfliGetLengthSymbolExtraBits(i) * ll_counts[i];
        }
        for (i = 0; i < 30; i++)
        {
            result += d_lengths[i] * d_counts[i];
            result += ZopfliGetDistSymbolExtraBits(i) * d_counts[i];
        }
        result += ll_lengths[256]; /*end symbol*/
        return result;
    }
}

/*
Comparator for sorting the leaves. Has the function signature for qsort.
*/
static int LeafComparator(const void *a, const void *b)
{
    return ((const Node *)a)->weight - ((const Node *)b)->weight;
}

/*
Converts result of boundary package-merge to the bitlengths. The result in the
last chain of the last list contains the amount of active leaves in each list.
chain: Chain to extract the bit length from (last chain from last list).
*/
static void ExtractBitLengths(Node *chain, Node *leaves, unsigned *bitlengths)
{
    int counts[16] = {0};
    unsigned end = 16;
    unsigned ptr = 15;
    unsigned value = 1;
    Node *node;
    int val;

    for (node = chain; node; node = node->tail)
    {
        counts[--end] = node->count;
    }

    val = counts[15];
    while (ptr >= end)
    {
        for (; val > counts[ptr - 1]; val--)
        {
            bitlengths[leaves[val - 1].count] = value;
        }
        ptr--;
        value++;
    }
}

/*
Initializes a chain node with the given values and marks it as in use.
*/
static void InitNode(size_t weight, int count, Node *tail, Node *node)
{
    node->weight = weight;
    node->count = count;
    node->tail = tail;
}

/*
Initializes each list with as lookahead chains the two leaves with lowest
weights.
*/
static void InitLists(
    NodePool *pool, const Node *leaves, int maxbits, Node *(*lists)[2])
{
    int i;
    Node *node0 = pool->next++;
    Node *node1 = pool->next++;
    InitNode(leaves[0].weight, 1, 0, node0);
    InitNode(leaves[1].weight, 2, 0, node1);
    for (i = 0; i < maxbits; i++)
    {
        lists[i][0] = node0;
        lists[i][1] = node1;
    }
}

static void BoundaryPMFinal(Node *(*lists)[2],
                            Node *leaves, int numsymbols, NodePool *pool, int index)
{
    int lastcount = lists[index][1]->count; /* Count of last chain of list. */

    size_t sum = lists[index - 1][0]->weight + lists[index - 1][1]->weight;

    if (lastcount < numsymbols && sum > leaves[lastcount].weight)
    {
        Node *newchain = pool->next;
        Node *oldchain = lists[index][1]->tail;

        lists[index][1] = newchain;
        newchain->count = lastcount + 1;
        newchain->tail = oldchain;
    }
    else
    {
        lists[index][1]->tail = lists[index - 1][1];
    }
}

/*
Performs a Boundary Package-Merge step. Puts a new chain in the given list. The
new chain is, depending on the weights, a leaf or a combination of two chains
from the previous list.
lists: The lists of chains.
maxbits: Number of lists.
leaves: The leaves, one per symbol.
numsymbols: Number of leaves.
pool: the node memory pool.
index: The index of the list in which a new chain or leaf is required.
*/
static void BoundaryPM(Node *(*lists)[2], Node *leaves, int numsymbols,
                       NodePool *pool, int index)
{
    Node *newchain;
    Node *oldchain;
    int lastcount = lists[index][1]->count; /* Count of last chain of list. */

    if (index == 0 && lastcount >= numsymbols)
        return;

    newchain = pool->next++;
    oldchain = lists[index][1];

    /* These are set up before the recursive calls below, so that there is a list
    pointing to the new node, to let the garbage collection know it's in use. */
    lists[index][0] = oldchain;
    lists[index][1] = newchain;

    if (index == 0)
    {
        /* New leaf node in list 0. */
        InitNode(leaves[lastcount].weight, lastcount + 1, 0, newchain);
    }
    else
    {
        size_t sum = lists[index - 1][0]->weight + lists[index - 1][1]->weight;
        if (lastcount < numsymbols && sum > leaves[lastcount].weight)
        {
            /* New leaf inserted in list, so count is incremented. */
            InitNode(leaves[lastcount].weight, lastcount + 1, oldchain->tail,
                     newchain);
        }
        else
        {
            InitNode(sum, lastcount, lists[index - 1][1], newchain);
            /* Two lookahead chains of previous list used up, create new ones. */
            BoundaryPM(lists, leaves, numsymbols, pool, index - 1);
            BoundaryPM(lists, leaves, numsymbols, pool, index - 1);
        }
    }
}

/*
Outputs minimum-redundancy length-limited code bitlengths for symbols with the
given counts. The bitlengths are limited by maxbits.

The output is tailored for DEFLATE: symbols that never occur, get a bit length
of 0, and if only a single symbol occurs at least once, its bitlength will be 1,
and not 0 as would theoretically be needed for a single symbol.

frequencies: The amount of occurrences of each symbol.
n: The amount of symbols.
maxbits: Maximum bit length, inclusive.
bitlengths: Output, the bitlengths for the symbol prefix codes.
return: 0 for OK, non-0 for error.
*/
int ZopfliLengthLimitedCodeLengths(
    const size_t *frequencies, int n, int maxbits, unsigned *bitlengths)
{
    NodePool pool;
    int i;
    int numsymbols = 0; /* Amount of symbols with frequency > 0. */
    int numBoundaryPMRuns;
    Node *nodes;

    /* Array of lists of chains. Each list requires only two lookahead chains at
    a time, so each list is a array of two Node*'s. */
    Node *(*lists)[2];

    /* One leaf per symbol. Only numsymbols leaves will be used. */
    Node *leaves = (Node *)malloc(n * sizeof(*leaves));

    /* Initialize all bitlengths at 0. */
    for (i = 0; i < n; i++)
    {
        bitlengths[i] = 0;
    }

    /* Count used symbols and place them in the leaves. */
    for (i = 0; i < n; i++)
    {
        if (frequencies[i])
        {
            leaves[numsymbols].weight = frequencies[i];
            leaves[numsymbols].count = i; /* Index of symbol this leaf represents. */
            numsymbols++;
        }
    }

    /* Check special cases and error conditions. */
    if ((1 << maxbits) < numsymbols)
    {
        free(leaves);
        return 1; /* Error, too few maxbits to represent symbols. */
    }
    if (numsymbols == 0)
    {
        free(leaves);
        return 0; /* No symbols at all. OK. */
    }
    if (numsymbols == 1)
    {
        bitlengths[leaves[0].count] = 1;
        free(leaves);
        return 0; /* Only one symbol, give it bitlength 1, not 0. OK. */
    }
    if (numsymbols == 2)
    {
        bitlengths[leaves[0].count]++;
        bitlengths[leaves[1].count]++;
        free(leaves);
        return 0;
    }

    /* Sort the leaves from lightest to heaviest. Add count into the same
    variable for stable sorting. */
    for (i = 0; i < numsymbols; i++)
    {
        if (leaves[i].weight >=
            ((size_t)1 << (sizeof(leaves[0].weight) * CHAR_BIT - 9)))
        {
            free(leaves);
            return 1; /* Error, we need 9 bits for the count. */
        }
        leaves[i].weight = (leaves[i].weight << 9) | leaves[i].count;
    }
    qsort(leaves, numsymbols, sizeof(Node), LeafComparator);
    for (i = 0; i < numsymbols; i++)
    {
        leaves[i].weight >>= 9;
    }

    if (numsymbols - 1 < maxbits)
    {
        maxbits = numsymbols - 1;
    }

    /* Initialize node memory pool. */
    nodes = (Node *)malloc(maxbits * 2 * numsymbols * sizeof(Node));
    pool.next = nodes;

    lists = (Node * (*)[2]) malloc(maxbits * sizeof(*lists));
    InitLists(&pool, leaves, maxbits, lists);

    /* In the last list, 2 * numsymbols - 2 active chains need to be created. Two
    are already created in the initialization. Each BoundaryPM run creates one. */
    numBoundaryPMRuns = 2 * numsymbols - 4;
    for (i = 0; i < numBoundaryPMRuns - 1; i++)
    {
        BoundaryPM(lists, leaves, numsymbols, &pool, maxbits - 1);
    }
    BoundaryPMFinal(lists, leaves, numsymbols, &pool, maxbits - 1);

    ExtractBitLengths(lists[maxbits - 1][1], leaves, bitlengths);

    free(lists);
    free(leaves);
    free(nodes);
    return 0; /* OK. */
}

/*
Calculates the bitlengths for the Huffman tree, based on the counts of each
symbol.
*/
void ZopfliCalculateBitLengths(const size_t *count, size_t n, int maxbits,
                               unsigned *bitlengths)
{
    int error = ZopfliLengthLimitedCodeLengths(count, n, maxbits, bitlengths);
    (void)error;
    assert(!error);
}

/*
Adds bits, like AddBits, but the order is inverted. The deflate specification
uses both orders in one standard.
*/
static void AddHuffmanBits(unsigned symbol, unsigned length,
                           unsigned char *bp, unsigned char **out,
                           size_t *outsize)
{
    /* TODO(lode): make more efficient (add more bits at once). */
    unsigned i;
    for (i = 0; i < length; i++)
    {
        unsigned bit = (symbol >> (length - i - 1)) & 1;
        if (*bp == 0)
            ZOPFLI_APPEND_DATA(0, out, outsize);
        (*out)[*outsize - 1] |= bit << *bp;
        *bp = (*bp + 1) & 7;
    }
}

static void AddBits(unsigned symbol, unsigned length,
                    unsigned char *bp, unsigned char **out, size_t *outsize)
{
    /* TODO(lode): make more efficient (add more bits at once). */
    unsigned i;
    for (i = 0; i < length; i++)
    {
        unsigned bit = (symbol >> i) & 1;
        if (*bp == 0)
            ZOPFLI_APPEND_DATA(0, out, outsize);
        (*out)[*outsize - 1] |= bit << *bp;
        *bp = (*bp + 1) & 7;
    }
}

/*
Converts a series of Huffman tree bitlengths, to the bit values of the symbols.
*/
void ZopfliLengthsToSymbols(const unsigned *lengths, size_t n, unsigned maxbits,
                            unsigned *symbols)
{
    size_t *bl_count = (size_t *)malloc(sizeof(size_t) * (maxbits + 1));
    size_t *next_code = (size_t *)malloc(sizeof(size_t) * (maxbits + 1));
    unsigned bits, i;
    unsigned code;

    for (i = 0; i < n; i++)
    {
        symbols[i] = 0;
    }

    /* 1) Count the number of codes for each code length. Let bl_count[N] be the
    number of codes of length N, N >= 1. */
    for (bits = 0; bits <= maxbits; bits++)
    {
        bl_count[bits] = 0;
    }
    for (i = 0; i < n; i++)
    {
        assert(lengths[i] <= maxbits);
        bl_count[lengths[i]]++;
    }
    /* 2) Find the numerical value of the smallest code for each code length. */
    code = 0;
    bl_count[0] = 0;
    for (bits = 1; bits <= maxbits; bits++)
    {
        code = (code + bl_count[bits - 1]) << 1;
        next_code[bits] = code;
    }
    /* 3) Assign numerical values to all codes, using consecutive values for all
    codes of the same length with the base values determined at step 2. */
    for (i = 0; i < n; i++)
    {
        unsigned len = lengths[i];
        if (len != 0)
        {
            symbols[i] = next_code[len];
            next_code[len]++;
        }
    }

    free(bl_count);
    free(next_code);
}

/*
Encodes the Huffman tree and returns how many bits its encoding takes. If out
is a null pointer, only returns the size and runs faster.
*/
static size_t EncodeTree(const unsigned *ll_lengths,
                         const unsigned *d_lengths,
                         int use_16, int use_17, int use_18,
                         unsigned char *bp,
                         unsigned char **out, size_t *outsize)
{
    unsigned lld_total; /* Total amount of literal, length, distance codes. */
    /* Runlength encoded version of lengths of litlen and dist trees. */
    unsigned *rle = 0;
    unsigned *rle_bits = 0;   /* Extra bits for rle values 16, 17 and 18. */
    size_t rle_size = 0;      /* Size of rle array. */
    size_t rle_bits_size = 0; /* Should have same value as rle_size. */
    unsigned hlit = 29;       /* 286 - 257 */
    unsigned hdist = 29;      /* 32 - 1, but gzip does not like hdist > 29.*/
    unsigned hclen;
    unsigned hlit2;
    size_t i, j;
    size_t clcounts[19];
    unsigned clcl[19]; /* Code length code lengths. */
    unsigned clsymbols[19];
    /* The order in which code length code lengths are encoded as per deflate. */
    static const unsigned order[19] = {
        16, 17, 18, 0, 8, 7, 9, 6, 10, 5, 11, 4, 12, 3, 13, 2, 14, 1, 15};
    int size_only = !out;
    size_t result_size = 0;

    for (i = 0; i < 19; i++)
        clcounts[i] = 0;

    /* Trim zeros. */
    while (hlit > 0 && ll_lengths[257 + hlit - 1] == 0)
        hlit--;
    while (hdist > 0 && d_lengths[1 + hdist - 1] == 0)
        hdist--;
    hlit2 = hlit + 257;

    lld_total = hlit2 + hdist + 1;

    for (i = 0; i < lld_total; i++)
    {
        /* This is an encoding of a huffman tree, so now the length is a symbol */
        unsigned char symbol = i < hlit2 ? ll_lengths[i] : d_lengths[i - hlit2];
        unsigned count = 1;
        if (use_16 || (symbol == 0 && (use_17 || use_18)))
        {
            for (j = i + 1; j < lld_total && symbol ==
                                                 (j < hlit2 ? ll_lengths[j] : d_lengths[j - hlit2]);
                 j++)
            {
                count++;
            }
        }
        i += count - 1;

        /* Repetitions of zeroes */
        if (symbol == 0 && count >= 3)
        {
            if (use_18)
            {
                while (count >= 11)
                {
                    unsigned count2 = count > 138 ? 138 : count;
                    if (!size_only)
                    {
                        ZOPFLI_APPEND_DATA(18, &rle, &rle_size);
                        ZOPFLI_APPEND_DATA(count2 - 11, &rle_bits, &rle_bits_size);
                    }
                    clcounts[18]++;
                    count -= count2;
                }
            }
            if (use_17)
            {
                while (count >= 3)
                {
                    unsigned count2 = count > 10 ? 10 : count;
                    if (!size_only)
                    {
                        ZOPFLI_APPEND_DATA(17, &rle, &rle_size);
                        ZOPFLI_APPEND_DATA(count2 - 3, &rle_bits, &rle_bits_size);
                    }
                    clcounts[17]++;
                    count -= count2;
                }
            }
        }

        /* Repetitions of any symbol */
        if (use_16 && count >= 4)
        {
            count--; /* Since the first one is hardcoded. */
            clcounts[symbol]++;
            if (!size_only)
            {
                ZOPFLI_APPEND_DATA(symbol, &rle, &rle_size);
                ZOPFLI_APPEND_DATA(0, &rle_bits, &rle_bits_size);
            }
            while (count >= 3)
            {
                unsigned count2 = count > 6 ? 6 : count;
                if (!size_only)
                {
                    ZOPFLI_APPEND_DATA(16, &rle, &rle_size);
                    ZOPFLI_APPEND_DATA(count2 - 3, &rle_bits, &rle_bits_size);
                }
                clcounts[16]++;
                count -= count2;
            }
        }

        /* No or insufficient repetition */
        clcounts[symbol] += count;
        while (count > 0)
        {
            if (!size_only)
            {
                ZOPFLI_APPEND_DATA(symbol, &rle, &rle_size);
                ZOPFLI_APPEND_DATA(0, &rle_bits, &rle_bits_size);
            }
            count--;
        }
    }

    ZopfliCalculateBitLengths(clcounts, 19, 7, clcl);
    if (!size_only)
        ZopfliLengthsToSymbols(clcl, 19, 7, clsymbols);

    hclen = 15;
    /* Trim zeros. */
    while (hclen > 0 && clcounts[order[hclen + 4 - 1]] == 0)
        hclen--;

    if (!size_only)
    {
        AddBits(hlit, 5, bp, out, outsize);
        AddBits(hdist, 5, bp, out, outsize);
        AddBits(hclen, 4, bp, out, outsize);

        for (i = 0; i < hclen + 4; i++)
        {
            AddBits(clcl[order[i]], 3, bp, out, outsize);
        }

        for (i = 0; i < rle_size; i++)
        {
            unsigned symbol = clsymbols[rle[i]];
            AddHuffmanBits(symbol, clcl[rle[i]], bp, out, outsize);
            /* Extra bits. */
            if (rle[i] == 16)
                AddBits(rle_bits[i], 2, bp, out, outsize);
            else if (rle[i] == 17)
                AddBits(rle_bits[i], 3, bp, out, outsize);
            else if (rle[i] == 18)
                AddBits(rle_bits[i], 7, bp, out, outsize);
        }
    }

    result_size += 14;              /* hlit, hdist, hclen bits */
    result_size += (hclen + 4) * 3; /* clcl bits */
    for (i = 0; i < 19; i++)
    {
        result_size += clcl[i] * clcounts[i];
    }
    /* Extra bits. */
    result_size += clcounts[16] * 2;
    result_size += clcounts[17] * 3;
    result_size += clcounts[18] * 7;

    /* Note: in case of "size_only" these are null pointers so no effect. */
    free(rle);
    free(rle_bits);

    return result_size;
}

/*
Gives the exact size of the tree, in bits, as it will be encoded in DEFLATE.
*/
static size_t CalculateTreeSize(const unsigned *ll_lengths,
                                const unsigned *d_lengths)
{
    size_t result = 0;
    int i;

    for (i = 0; i < 8; i++)
    {
        size_t size = EncodeTree(ll_lengths, d_lengths,
                                 i & 1, i & 2, i & 4,
                                 0, 0, 0);
        if (result == 0 || size < result)
            result = size;
    }

    return result;
}

/*
Ensures there are at least 2 distance codes to support buggy decoders.
Zlib 1.2.1 and below have a bug where it fails if there isn't at least 1
distance code (with length > 0), even though it's valid according to the
deflate spec to have 0 distance codes. On top of that, some mobile phones
require at least two distance codes. To support these decoders too (but
potentially at the cost of a few bytes), add dummy code lengths of 1.
References to this bug can be found in the changelog of
Zlib 1.2.2 and here: http://www.jonof.id.au/forum/index.php?topic=515.0.

d_lengths: the 32 lengths of the distance codes.
*/
static void PatchDistanceCodesForBuggyDecoders(unsigned *d_lengths)
{
    int num_dist_codes = 0; /* Amount of non-zero distance codes */
    int i;
    for (i = 0; i < 30 /* Ignore the two unused codes from the spec */; i++)
    {
        if (d_lengths[i])
            num_dist_codes++;
        if (num_dist_codes >= 2)
            return; /* Two or more codes is fine. */
    }

    if (num_dist_codes == 0)
    {
        d_lengths[0] = d_lengths[1] = 1;
    }
    else if (num_dist_codes == 1)
    {
        d_lengths[d_lengths[0] ? 1 : 0] = 1;
    }
}

/*
Tries out OptimizeHuffmanForRle for this block, if the result is smaller,
uses it, otherwise keeps the original. Returns size of encoded tree and data in
bits, not including the 3-bit block header.
*/
static double TryOptimizeHuffmanForRle(
    const ZopfliLZ77Store *lz77, size_t lstart, size_t lend,
    const size_t *ll_counts, const size_t *d_counts,
    unsigned *ll_lengths, unsigned *d_lengths)
{
    size_t ll_counts2[ZOPFLI_NUM_LL];
    size_t d_counts2[ZOPFLI_NUM_D];
    unsigned ll_lengths2[ZOPFLI_NUM_LL];
    unsigned d_lengths2[ZOPFLI_NUM_D];
    double treesize;
    double datasize;
    double treesize2;
    double datasize2;

    treesize = CalculateTreeSize(ll_lengths, d_lengths);
    datasize = CalculateBlockSymbolSizeGivenCounts(ll_counts, d_counts,
                                                   ll_lengths, d_lengths, lz77, lstart, lend);

    memcpy(ll_counts2, ll_counts, sizeof(ll_counts2));
    memcpy(d_counts2, d_counts, sizeof(d_counts2));
    OptimizeHuffmanForRle(ZOPFLI_NUM_LL, ll_counts2);
    OptimizeHuffmanForRle(ZOPFLI_NUM_D, d_counts2);
    ZopfliCalculateBitLengths(ll_counts2, ZOPFLI_NUM_LL, 15, ll_lengths2);
    ZopfliCalculateBitLengths(d_counts2, ZOPFLI_NUM_D, 15, d_lengths2);
    PatchDistanceCodesForBuggyDecoders(d_lengths2);

    treesize2 = CalculateTreeSize(ll_lengths2, d_lengths2);
    datasize2 = CalculateBlockSymbolSizeGivenCounts(ll_counts, d_counts,
                                                    ll_lengths2, d_lengths2, lz77, lstart, lend);

    if (treesize2 + datasize2 < treesize + datasize)
    {
        memcpy(ll_lengths, ll_lengths2, sizeof(ll_lengths2));
        memcpy(d_lengths, d_lengths2, sizeof(d_lengths2));
        return treesize2 + datasize2;
    }
    return treesize + datasize;
}

/* Gets the histogram of lit/len and dist symbols in the given range, using the
cumulative histograms, so faster than adding one by one for large range. Does
not add the one end symbol of value 256. */
static void ZopfliLZ77GetHistogramAt(const ZopfliLZ77Store *lz77, size_t lpos,
                                     size_t *ll_counts, size_t *d_counts)
{
    /* The real histogram is created by using the histogram for this chunk, but
    all superfluous values of this chunk subtracted. */
    size_t llpos = ZOPFLI_NUM_LL * (lpos / ZOPFLI_NUM_LL);
    size_t dpos = ZOPFLI_NUM_D * (lpos / ZOPFLI_NUM_D);
    size_t i;
    for (i = 0; i < ZOPFLI_NUM_LL; i++)
    {
        ll_counts[i] = lz77->ll_counts[llpos + i];
    }
    for (i = lpos + 1; i < llpos + ZOPFLI_NUM_LL && i < lz77->size; i++)
    {
        ll_counts[lz77->ll_symbol[i]]--;
    }
    for (i = 0; i < ZOPFLI_NUM_D; i++)
    {
        d_counts[i] = lz77->d_counts[dpos + i];
    }
    for (i = lpos + 1; i < dpos + ZOPFLI_NUM_D && i < lz77->size; i++)
    {
        if (lz77->dists[i] != 0)
            d_counts[lz77->d_symbol[i]]--;
    }
}

void ZopfliLZ77GetHistogram(const ZopfliLZ77Store *lz77,
                            size_t lstart, size_t lend,
                            size_t *ll_counts, size_t *d_counts)
{
    size_t i;
    if (lstart + ZOPFLI_NUM_LL * 3 > lend)
    {
        memset(ll_counts, 0, sizeof(*ll_counts) * ZOPFLI_NUM_LL);
        memset(d_counts, 0, sizeof(*d_counts) * ZOPFLI_NUM_D);
        for (i = lstart; i < lend; i++)
        {
            ll_counts[lz77->ll_symbol[i]]++;
            if (lz77->dists[i] != 0)
                d_counts[lz77->d_symbol[i]]++;
        }
    }
    else
    {
        /* Subtract the cumulative histograms at the end and the start to get the
        histogram for this range. */
        ZopfliLZ77GetHistogramAt(lz77, lend - 1, ll_counts, d_counts);
        if (lstart > 0)
        {
            size_t ll_counts2[ZOPFLI_NUM_LL];
            size_t d_counts2[ZOPFLI_NUM_D];
            ZopfliLZ77GetHistogramAt(lz77, lstart - 1, ll_counts2, d_counts2);

            for (i = 0; i < ZOPFLI_NUM_LL; i++)
            {
                ll_counts[i] -= ll_counts2[i];
            }
            for (i = 0; i < ZOPFLI_NUM_D; i++)
            {
                d_counts[i] -= d_counts2[i];
            }
        }
    }
}

/*
Calculates the bit lengths for the symbols for dynamic blocks. Chooses bit
lengths that give the smallest size of tree encoding + encoding of all the
symbols to have smallest output size. This are not necessarily the ideal Huffman
bit lengths. Returns size of encoded tree and data in bits, not including the
3-bit block header.
*/
static double GetDynamicLengths(const ZopfliLZ77Store *lz77,
                                size_t lstart, size_t lend,
                                unsigned *ll_lengths, unsigned *d_lengths)
{
    size_t ll_counts[ZOPFLI_NUM_LL];
    size_t d_counts[ZOPFLI_NUM_D];

    ZopfliLZ77GetHistogram(lz77, lstart, lend, ll_counts, d_counts);
    ll_counts[256] = 1; /* End symbol. */
    ZopfliCalculateBitLengths(ll_counts, ZOPFLI_NUM_LL, 15, ll_lengths);
    ZopfliCalculateBitLengths(d_counts, ZOPFLI_NUM_D, 15, d_lengths);
    PatchDistanceCodesForBuggyDecoders(d_lengths);
    return TryOptimizeHuffmanForRle(
        lz77, lstart, lend, ll_counts, d_counts, ll_lengths, d_lengths);
}

static void GetFixedTree(unsigned *ll_lengths, unsigned *d_lengths)
{
    size_t i;
    for (i = 0; i < 144; i++)
        ll_lengths[i] = 8;
    for (i = 144; i < 256; i++)
        ll_lengths[i] = 9;
    for (i = 256; i < 280; i++)
        ll_lengths[i] = 7;
    for (i = 280; i < 288; i++)
        ll_lengths[i] = 8;
    for (i = 0; i < 32; i++)
        d_lengths[i] = 5;
}

size_t ZopfliLZ77GetByteRange(const ZopfliLZ77Store *lz77,
                              size_t lstart, size_t lend)
{
    size_t l = lend - 1;
    if (lstart == lend)
        return 0;
    return lz77->pos[l] + ((lz77->dists[l] == 0) ? 1 : lz77->litlens[l]) - lz77->pos[lstart];
}

/*
Calculates size of the part after the header and tree of an LZ77 block, in bits.
*/
static size_t CalculateBlockSymbolSize(const unsigned *ll_lengths,
                                       const unsigned *d_lengths,
                                       const ZopfliLZ77Store *lz77,
                                       size_t lstart, size_t lend)
{
    if (lstart + ZOPFLI_NUM_LL * 3 > lend)
    {
        return CalculateBlockSymbolSizeSmall(
            ll_lengths, d_lengths, lz77, lstart, lend);
    }
    else
    {
        size_t ll_counts[ZOPFLI_NUM_LL];
        size_t d_counts[ZOPFLI_NUM_D];
        ZopfliLZ77GetHistogram(lz77, lstart, lend, ll_counts, d_counts);
        return CalculateBlockSymbolSizeGivenCounts(
            ll_counts, d_counts, ll_lengths, d_lengths, lz77, lstart, lend);
    }
}

/*
Calculates block size in bits.
litlens: lz77 lit/lengths
dists: ll77 distances
lstart: start of block
lend: end of block (not inclusive)
*/
double ZopfliCalculateBlockSize(const ZopfliLZ77Store *lz77,
                                size_t lstart, size_t lend, int btype)
{
    unsigned ll_lengths[ZOPFLI_NUM_LL];
    unsigned d_lengths[ZOPFLI_NUM_D];

    double result = 3; /* bfinal and btype bits */

    if (btype == 0)
    {
        size_t length = ZopfliLZ77GetByteRange(lz77, lstart, lend);
        size_t rem = length % 65535;
        size_t blocks = length / 65535 + (rem ? 1 : 0);
        /* An uncompressed block must actually be split into multiple blocks if it's
           larger than 65535 bytes long. Eeach block header is 5 bytes: 3 bits,
           padding, LEN and NLEN (potential less padding for first one ignored). */
        return blocks * 5 * 8 + length * 8;
    }
    if (btype == 1)
    {
        GetFixedTree(ll_lengths, d_lengths);
        result += CalculateBlockSymbolSize(
            ll_lengths, d_lengths, lz77, lstart, lend);
    }
    else
    {
        result += GetDynamicLengths(lz77, lstart, lend, ll_lengths, d_lengths);
    }

    return result;
}

/*
Calculates block size in bits, automatically using the best btype.
*/
double ZopfliCalculateBlockSizeAutoType(const ZopfliLZ77Store *lz77,
                                        size_t lstart, size_t lend)
{
    double uncompressedcost = ZopfliCalculateBlockSize(lz77, lstart, lend, 0);
    /* Don't do the expensive fixed cost calculation for larger blocks that are
       unlikely to use it. */
    double fixedcost = (lz77->size > 1000) ? uncompressedcost : ZopfliCalculateBlockSize(lz77, lstart, lend, 1);
    double dyncost = ZopfliCalculateBlockSize(lz77, lstart, lend, 2);
    return (uncompressedcost < fixedcost && uncompressedcost < dyncost)
               ? uncompressedcost
               : (fixedcost < dyncost ? fixedcost : dyncost);
}

/*
Returns estimated cost of a block in bits.  It includes the size to encode the
tree and the size to encode all literal, length and distance symbols and their
extra bits.

litlens: lz77 lit/lengths
dists: ll77 distances
lstart: start of block
lend: end of block (not inclusive)
*/
static double EstimateCost(const ZopfliLZ77Store *lz77,
                           size_t lstart, size_t lend)
{
    return ZopfliCalculateBlockSizeAutoType(lz77, lstart, lend);
}

/*
Gets the cost which is the sum of the cost of the left and the right section
of the data.
type: FindMinimumFun
*/
static double SplitCost(size_t i, void *context)
{
    SplitCostContext *c = (SplitCostContext *)context;
    return EstimateCost(c->lz77, c->start, i) + EstimateCost(c->lz77, i, c->end);
}

/* Gets the amount of extra bits for the given length, cfr. the DEFLATE spec. */
static int ZopfliGetLengthExtraBits(int l)
{
    static const int table[259] = {
        0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1, 1, 1, 1, 1, 1, 1, 1,
        2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2,
        3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3,
        3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3,
        4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4,
        4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4,
        4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4,
        4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4,
        5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5,
        5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5,
        5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5,
        5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5,
        5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5,
        5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5,
        5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5,
        5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 0};
    return table[l];
}

/* Gets the amount of extra bits for the given dist, cfr. the DEFLATE spec. */
static int ZopfliGetDistExtraBits(int dist)
{
    if (dist < 5)
        return 0;
    return (31 ^ __builtin_clz(dist - 1)) - 1; /* log2(dist - 1) - 1 */
}

/*
Cost model based on symbol statistics.
type: CostModelFun
*/
static double GetCostStat(unsigned litlen, unsigned dist, void *context)
{
    SymbolStats *stats = (SymbolStats *)context;
    if (dist == 0)
    {
        return stats->ll_symbols[litlen];
    }
    else
    {
        int lsym = ZopfliGetLengthSymbol(litlen);
        int lbits = ZopfliGetLengthExtraBits(litlen);
        int dsym = ZopfliGetDistSymbol(dist);
        int dbits = ZopfliGetDistExtraBits(dist);
        return lbits + dbits + stats->ll_symbols[lsym] + stats->d_symbols[dsym];
    }
}

/*
Cost model which should exactly match fixed tree.
type: CostModelFun
*/
static double GetCostFixed(unsigned litlen, unsigned dist, void *unused)
{
    (void)unused;
    if (dist == 0)
    {
        if (litlen <= 143)
            return 8;
        else
            return 9;
    }
    else
    {
        int dbits = ZopfliGetDistExtraBits(dist);
        int lbits = ZopfliGetLengthExtraBits(litlen);
        int lsym = ZopfliGetLengthSymbol(litlen);
        int cost = 0;
        if (lsym <= 279)
            cost += 7;
        else
            cost += 8;
        cost += 5; /* Every dist symbol has length 5. */
        return cost + dbits + lbits;
    }
}

/*
bp = bitpointer, always in range [0, 7].
The outsize is number of necessary bytes to encode the bits.
Given the value of bp and the amount of bytes, the amount of bits represented
is not simply bytesize * 8 + bp because even representing one bit requires a
whole byte. It is: (bp == 0) ? (bytesize * 8) : ((bytesize - 1) * 8 + bp)
*/
static void AddBit(int bit,
                   unsigned char *bp, unsigned char **out, size_t *outsize)
{
    if (*bp == 0)
        ZOPFLI_APPEND_DATA(0, out, outsize);
    (*out)[*outsize - 1] |= bit << *bp;
    *bp = (*bp + 1) & 7;
}

/* Since an uncompressed block can be max 65535 in size, it actually adds
multible blocks if needed. */
static void AddNonCompressedBlock(const ZopfliOptions *options, int final,
                                  const unsigned char *in, size_t instart,
                                  size_t inend,
                                  unsigned char *bp,
                                  unsigned char **out, size_t *outsize)
{
    size_t pos = instart;
    (void)options;
    for (;;)
    {
        size_t i;
        unsigned short blocksize = 65535;
        unsigned short nlen;
        int currentfinal;

        if (pos + blocksize > inend)
            blocksize = inend - pos;
        currentfinal = pos + blocksize >= inend;

        nlen = ~blocksize;

        AddBit(final && currentfinal, bp, out, outsize);
        /* BTYPE 00 */
        AddBit(0, bp, out, outsize);
        AddBit(0, bp, out, outsize);

        /* Any bits of input up to the next byte boundary are ignored. */
        *bp = 0;

        ZOPFLI_APPEND_DATA(blocksize % 256, out, outsize);
        ZOPFLI_APPEND_DATA((blocksize / 256) % 256, out, outsize);
        ZOPFLI_APPEND_DATA(nlen % 256, out, outsize);
        ZOPFLI_APPEND_DATA((nlen / 256) % 256, out, outsize);

        for (i = 0; i < blocksize; i++)
        {
            ZOPFLI_APPEND_DATA(in[pos + i], out, outsize);
        }

        if (currentfinal)
            break;
        pos += blocksize;
    }
}

/* Gets value of the extra bits for the given length, cfr. the DEFLATE spec. */
static int ZopfliGetLengthExtraBitsValue(int l)
{
    static const int table[259] = {
        0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1, 0, 1, 0, 1, 0, 1, 0, 1, 2, 3, 0,
        1, 2, 3, 0, 1, 2, 3, 0, 1, 2, 3, 0, 1, 2, 3, 4, 5, 6, 7, 0, 1, 2, 3, 4, 5,
        6, 7, 0, 1, 2, 3, 4, 5, 6, 7, 0, 1, 2, 3, 4, 5, 6, 7, 0, 1, 2, 3, 4, 5, 6,
        7, 8, 9, 10, 11, 12, 13, 14, 15, 0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12,
        13, 14, 15, 0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 0, 1, 2,
        3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 0, 1, 2, 3, 4, 5, 6, 7, 8, 9,
        10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28,
        29, 30, 31, 0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17,
        18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 0, 1, 2, 3, 4, 5, 6,
        7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26,
        27, 28, 29, 30, 31, 0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15,
        16, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 0};
    return table[l];
}

/* Gets value of the extra bits for the given dist, cfr. the DEFLATE spec. */
static int ZopfliGetDistExtraBitsValue(int dist)
{
    if (dist < 5)
    {
        return 0;
    }
    else
    {
        int l = 31 ^ __builtin_clz(dist - 1); /* log2(dist - 1) */
        return (dist - (1 + (1 << l))) & ((1 << (l - 1)) - 1);
    }
}

/*
Adds all lit/len and dist codes from the lists as huffman symbols. Does not add
end code 256. expected_data_size is the uncompressed block size, used for
assert, but you can set it to 0 to not do the assertion.
*/
static void AddLZ77Data(const ZopfliLZ77Store *lz77,
                        size_t lstart, size_t lend,
                        size_t expected_data_size,
                        const unsigned *ll_symbols, const unsigned *ll_lengths,
                        const unsigned *d_symbols, const unsigned *d_lengths,
                        unsigned char *bp,
                        unsigned char **out, size_t *outsize)
{
    size_t testlength = 0;
    size_t i;

    for (i = lstart; i < lend; i++)
    {
        unsigned dist = lz77->dists[i];
        unsigned litlen = lz77->litlens[i];
        if (dist == 0)
        {
            assert(litlen < 256);
            assert(ll_lengths[litlen] > 0);
            AddHuffmanBits(ll_symbols[litlen], ll_lengths[litlen], bp, out, outsize);
            testlength++;
        }
        else
        {
            unsigned lls = ZopfliGetLengthSymbol(litlen);
            unsigned ds = ZopfliGetDistSymbol(dist);
            assert(litlen >= 3 && litlen <= 288);
            assert(ll_lengths[lls] > 0);
            assert(d_lengths[ds] > 0);
            AddHuffmanBits(ll_symbols[lls], ll_lengths[lls], bp, out, outsize);
            AddBits(ZopfliGetLengthExtraBitsValue(litlen),
                    ZopfliGetLengthExtraBits(litlen),
                    bp, out, outsize);
            AddHuffmanBits(d_symbols[ds], d_lengths[ds], bp, out, outsize);
            AddBits(ZopfliGetDistExtraBitsValue(dist),
                    ZopfliGetDistExtraBits(dist),
                    bp, out, outsize);
            testlength += litlen;
        }
    }
    assert(expected_data_size == 0 || testlength == expected_data_size);
}

static void AddDynamicTree(const unsigned *ll_lengths,
                           const unsigned *d_lengths,
                           unsigned char *bp,
                           unsigned char **out, size_t *outsize)
{
    int i;
    int best = 0;
    size_t bestsize = 0;

    for (i = 0; i < 8; i++)
    {
        size_t size = EncodeTree(ll_lengths, d_lengths,
                                 i & 1, i & 2, i & 4,
                                 0, 0, 0);
        if (bestsize == 0 || size < bestsize)
        {
            bestsize = size;
            best = i;
        }
    }

    EncodeTree(ll_lengths, d_lengths,
               best & 1, best & 2, best & 4,
               bp, out, outsize);
}

/*
Adds a deflate block with the given LZ77 data to the output.
options: global program options
btype: the block type, must be 1 or 2
final: whether to set the "final" bit on this block, must be the last block
litlens: literal/length array of the LZ77 data, in the same format as in
ZopfliLZ77Store.
dists: distance array of the LZ77 data, in the same format as in
ZopfliLZ77Store.
lstart: where to start in the LZ77 data
lend: where to end in the LZ77 data (not inclusive)
expected_data_size: the uncompressed block size, used for assert, but you can
set it to 0 to not do the assertion.
bp: output bit pointer
out: dynamic output array to append to
outsize: dynamic output array size
*/
static void AddLZ77Block(const ZopfliOptions *options, int btype, int final,
                         const ZopfliLZ77Store *lz77,
                         size_t lstart, size_t lend,
                         size_t expected_data_size,
                         unsigned char *bp,
                         unsigned char **out, size_t *outsize)
{
    unsigned ll_lengths[ZOPFLI_NUM_LL];
    unsigned d_lengths[ZOPFLI_NUM_D];
    unsigned ll_symbols[ZOPFLI_NUM_LL];
    unsigned d_symbols[ZOPFLI_NUM_D];
    size_t detect_block_size = *outsize;
    size_t compressed_size;
    size_t uncompressed_size = 0;
    size_t i;
    if (btype == 0)
    {
        size_t length = ZopfliLZ77GetByteRange(lz77, lstart, lend);
        size_t pos = lstart == lend ? 0 : lz77->pos[lstart];
        size_t end = pos + length;
        AddNonCompressedBlock(options, final,
                              lz77->data, pos, end, bp, out, outsize);
        return;
    }

    AddBit(final, bp, out, outsize);
    AddBit(btype & 1, bp, out, outsize);
    AddBit((btype & 2) >> 1, bp, out, outsize);

    if (btype == 1)
    {
        /* Fixed block. */
        GetFixedTree(ll_lengths, d_lengths);
    }
    else
    {
        /* Dynamic block. */
        unsigned detect_tree_size;
        assert(btype == 2);

        GetDynamicLengths(lz77, lstart, lend, ll_lengths, d_lengths);

        detect_tree_size = *outsize;
        AddDynamicTree(ll_lengths, d_lengths, bp, out, outsize);
        if (options->verbose)
        {
            fprintf(stderr, "treesize: %d\n", (int)(*outsize - detect_tree_size));
        }
    }

    ZopfliLengthsToSymbols(ll_lengths, ZOPFLI_NUM_LL, 15, ll_symbols);
    ZopfliLengthsToSymbols(d_lengths, ZOPFLI_NUM_D, 15, d_symbols);

    detect_block_size = *outsize;
    AddLZ77Data(lz77, lstart, lend, expected_data_size,
                ll_symbols, ll_lengths, d_symbols, d_lengths,
                bp, out, outsize);
    /* End symbol. */
    AddHuffmanBits(ll_symbols[256], ll_lengths[256], bp, out, outsize);

    for (i = lstart; i < lend; i++)
    {
        uncompressed_size += lz77->dists[i] == 0 ? 1 : lz77->litlens[i];
    }
    compressed_size = *outsize - detect_block_size;
    if (options->verbose)
    {
        fprintf(stderr, "compressed block size: %d (%dk) (unc: %d)\n",
                (int)compressed_size, (int)(compressed_size / 1024),
                (int)(uncompressed_size));
    }
}

/*
Returns the length up to which could be stored in the cache.
*/
unsigned ZopfliMaxCachedSublen(const ZopfliLongestMatchCache *lmc,
                               size_t pos, size_t length)
{
    unsigned char *cache;
    cache = &lmc->sublen[ZOPFLI_CACHE_LENGTH * pos * 3];
    (void)length;
    if (cache[1] == 0 && cache[2] == 0)
        return 0; /* No sublen cached. */
    return cache[(ZOPFLI_CACHE_LENGTH - 1) * 3] + 3;
}

void ZopfliSublenToCache(const unsigned short *sublen,
                         size_t pos, size_t length,
                         ZopfliLongestMatchCache *lmc)
{
    size_t i;
    size_t j = 0;
    unsigned bestlength = 0;
    unsigned char *cache;

    cache = &lmc->sublen[ZOPFLI_CACHE_LENGTH * pos * 3];
    if (length < 3)
        return;
    for (i = 3; i <= length; i++)
    {
        if (i == length || sublen[i] != sublen[i + 1])
        {
            cache[j * 3] = i - 3;
            cache[j * 3 + 1] = sublen[i] % 256;
            cache[j * 3 + 2] = (sublen[i] >> 8) % 256;
            bestlength = i;
            j++;
            if (j >= ZOPFLI_CACHE_LENGTH)
                break;
        }
    }
    if (j < ZOPFLI_CACHE_LENGTH)
    {
        assert(bestlength == length);
        cache[(ZOPFLI_CACHE_LENGTH - 1) * 3] = bestlength - 3;
    }
    else
    {
        assert(bestlength <= length);
    }
    assert(bestlength == ZopfliMaxCachedSublen(lmc, pos, length));
}

/*
Stores the found sublen, distance and length in the longest match cache, if
possible.
*/
static void StoreInLongestMatchCache(ZopfliBlockState *s,
                                     size_t pos, size_t limit,
                                     const unsigned short *sublen,
                                     unsigned short distance, unsigned short length)
{
    /* The LMC cache starts at the beginning of the block rather than the
       beginning of the whole array. */
    size_t lmcpos = pos - s->blockstart;

    /* Length > 0 and dist 0 is invalid combination, which indicates on purpose
       that this cache value is not filled in yet. */
    unsigned char cache_available = s->lmc && (s->lmc->length[lmcpos] == 0 ||
                                               s->lmc->dist[lmcpos] != 0);

    if (s->lmc && limit == ZOPFLI_MAX_MATCH && sublen && !cache_available)
    {
        assert(s->lmc->length[lmcpos] == 1 && s->lmc->dist[lmcpos] == 0);
        s->lmc->dist[lmcpos] = length < ZOPFLI_MIN_MATCH ? 0 : distance;
        s->lmc->length[lmcpos] = length < ZOPFLI_MIN_MATCH ? 0 : length;
        assert(!(s->lmc->length[lmcpos] == 1 && s->lmc->dist[lmcpos] == 0));
        ZopfliSublenToCache(sublen, lmcpos, length, s->lmc);
    }
}

void ZopfliCacheToSublen(const ZopfliLongestMatchCache *lmc,
                         size_t pos, size_t length,
                         unsigned short *sublen)
{
    size_t i, j;
    unsigned maxlength = ZopfliMaxCachedSublen(lmc, pos, length);
    unsigned prevlength = 0;
    unsigned char *cache;
    if (length < 3)
        return;
    cache = &lmc->sublen[ZOPFLI_CACHE_LENGTH * pos * 3];
    for (j = 0; j < ZOPFLI_CACHE_LENGTH; j++)
    {
        unsigned length = cache[j * 3] + 3;
        unsigned dist = cache[j * 3 + 1] + 256 * cache[j * 3 + 2];
        for (i = prevlength; i <= length; i++)
        {
            sublen[i] = dist;
        }
        if (length == maxlength)
            break;
        prevlength = length + 1;
    }
}

/*
Gets distance, length and sublen values from the cache if possible.
Returns 1 if it got the values from the cache, 0 if not.
Updates the limit value to a smaller one if possible with more limited
information from the cache.
*/
static int TryGetFromLongestMatchCache(ZopfliBlockState *s,
                                       size_t pos, size_t *limit,
                                       unsigned short *sublen, unsigned short *distance, unsigned short *length)
{
    /* The LMC cache starts at the beginning of the block rather than the
       beginning of the whole array. */
    size_t lmcpos = pos - s->blockstart;

    /* Length > 0 and dist 0 is invalid combination, which indicates on purpose
       that this cache value is not filled in yet. */
    unsigned char cache_available = s->lmc && (s->lmc->length[lmcpos] == 0 ||
                                               s->lmc->dist[lmcpos] != 0);
    unsigned char limit_ok_for_cache = cache_available &&
                                       (*limit == ZOPFLI_MAX_MATCH || s->lmc->length[lmcpos] <= *limit ||
                                        (sublen && ZopfliMaxCachedSublen(s->lmc,
                                                                         lmcpos, s->lmc->length[lmcpos]) >= *limit));

    if (s->lmc && limit_ok_for_cache && cache_available)
    {
        if (!sublen || s->lmc->length[lmcpos] <= ZopfliMaxCachedSublen(s->lmc, lmcpos, s->lmc->length[lmcpos]))
        {
            *length = s->lmc->length[lmcpos];
            if (*length > *limit)
                *length = *limit;
            if (sublen)
            {
                ZopfliCacheToSublen(s->lmc, lmcpos, *length, sublen);
                *distance = sublen[*length];
                if (*limit == ZOPFLI_MAX_MATCH && *length >= ZOPFLI_MIN_MATCH)
                {
                    assert(sublen[*length] == s->lmc->dist[lmcpos]);
                }
            }
            else
            {
                *distance = s->lmc->dist[lmcpos];
            }
            return 1;
        }
        /* Can't use much of the cache, since the "sublens" need to be calculated,
           but at  least we already know when to stop. */
        *limit = s->lmc->length[lmcpos];
    }

    return 0;
}

/*
Finds how long the match of scan and match is. Can be used to find how many
bytes starting from scan, and from match, are equal. Returns the last byte
after scan, which is still equal to the correspondinb byte after match.
scan is the position to compare
match is the earlier position to compare.
end is the last possible byte, beyond which to stop looking.
safe_end is a few (8) bytes before end, for comparing multiple bytes at once.
*/
static const unsigned char *GetMatch(const unsigned char *scan,
                                     const unsigned char *match,
                                     const unsigned char *end,
                                     const unsigned char *safe_end)
{

    if (sizeof(size_t) == 8)
    {
        /* 8 checks at once per array bounds check (size_t is 64-bit). */
        while (scan < safe_end && *((size_t *)scan) == *((size_t *)match))
        {
            scan += 8;
            match += 8;
        }
    }
    else if (sizeof(unsigned int) == 4)
    {
        /* 4 checks at once per array bounds check (unsigned int is 32-bit). */
        while (scan < safe_end && *((unsigned int *)scan) == *((unsigned int *)match))
        {
            scan += 4;
            match += 4;
        }
    }
    else
    {
        /* do 8 checks at once per array bounds check. */
        while (scan < safe_end && *scan == *match && *++scan == *++match && *++scan == *++match && *++scan == *++match && *++scan == *++match && *++scan == *++match && *++scan == *++match && *++scan == *++match)
        {
            scan++;
            match++;
        }
    }

    /* The remaining few bytes. */
    while (scan != end && *scan == *match)
    {
        scan++;
        match++;
    }

    return scan;
}

/*
Finds the longest match (length and corresponding distance) for LZ77
compression.
Even when not using "sublen", it can be more efficient to provide an array,
because only then the caching is used.
array: the data
pos: position in the data to find the match for
size: size of the data
limit: limit length to maximum this value (default should be 258). This allows
finding a shorter dist for that length (= less extra bits). Must be
in the range [ZOPFLI_MIN_MATCH, ZOPFLI_MAX_MATCH].
sublen: output array of 259 elements, or null. Has, for each length, the
smallest distance required to reach this length. Only 256 of its 259 values
are used, the first 3 are ignored (the shortest length is 3. It is purely
for convenience that the array is made 3 longer).
*/
void ZopfliFindLongestMatch(ZopfliBlockState *s, const ZopfliHash *h,
                            const unsigned char *array,
                            size_t pos, size_t size, size_t limit,
                            unsigned short *sublen, unsigned short *distance, unsigned short *length)
{
    unsigned short hpos = pos & ZOPFLI_WINDOW_MASK, p, pp;
    unsigned short bestdist = 0;
    unsigned short bestlength = 1;
    const unsigned char *scan;
    const unsigned char *match;
    const unsigned char *arrayend;
    const unsigned char *arrayend_safe;
    int chain_counter = ZOPFLI_MAX_CHAIN_HITS; /* For quitting early. */

    unsigned dist = 0; /* Not unsigned short on purpose. */

    int *hhead = h->head;
    unsigned short *hprev = h->prev;
    int *hhashval = h->hashval;
    int hval = h->val;

    if (TryGetFromLongestMatchCache(s, pos, &limit, sublen, distance, length))
    {
        assert(pos + *length <= size);
        return;
    }

    assert(limit <= ZOPFLI_MAX_MATCH);
    assert(limit >= ZOPFLI_MIN_MATCH);
    assert(pos < size);

    if (size - pos < ZOPFLI_MIN_MATCH)
    {
        /* The rest of the code assumes there are at least ZOPFLI_MIN_MATCH bytes to
           try. */
        *length = 0;
        *distance = 0;
        return;
    }

    if (pos + limit > size)
    {
        limit = size - pos;
    }
    arrayend = &array[pos] + limit;
    arrayend_safe = arrayend - 8;

    assert(hval < 65536);

    pp = hhead[hval]; /* During the whole loop, p == hprev[pp]. */
    p = hprev[pp];

    assert(pp == hpos);

    dist = p < pp ? pp - p : ((ZOPFLI_WINDOW_SIZE - p) + pp);

    /* Go through all distances. */
    while (dist < ZOPFLI_WINDOW_SIZE)
    {
        unsigned short currentlength = 0;

        assert(p < ZOPFLI_WINDOW_SIZE);
        assert(p == hprev[pp]);
        assert(hhashval[p] == hval);

        if (dist > 0)
        {
            assert(pos < size);
            assert(dist <= pos);
            scan = &array[pos];
            match = &array[pos - dist];

            /* Testing the byte at position bestlength first, goes slightly faster. */
            if (pos + bestlength >= size || *(scan + bestlength) == *(match + bestlength))
            {

                unsigned short same0 = h->same[pos & ZOPFLI_WINDOW_MASK];
                if (same0 > 2 && *scan == *match)
                {
                    unsigned short same1 = h->same[(pos - dist) & ZOPFLI_WINDOW_MASK];
                    unsigned short same = same0 < same1 ? same0 : same1;
                    if (same > limit)
                        same = limit;
                    scan += same;
                    match += same;
                }
                scan = GetMatch(scan, match, arrayend, arrayend_safe);
                currentlength = scan - &array[pos]; /* The found length. */
            }

            if (currentlength > bestlength)
            {
                if (sublen)
                {
                    unsigned short j;
                    for (j = bestlength + 1; j <= currentlength; j++)
                    {
                        sublen[j] = dist;
                    }
                }
                bestdist = dist;
                bestlength = currentlength;
                if (currentlength >= limit)
                    break;
            }
        }

        /* Switch to the other hash once this will be more efficient. */
        if (hhead != h->head2 && bestlength >= h->same[hpos] &&
            h->val2 == h->hashval2[p])
        {
            /* Now use the hash that encodes the length and first byte. */
            hhead = h->head2;
            hprev = h->prev2;
            hhashval = h->hashval2;
            hval = h->val2;
        }

        pp = p;
        p = hprev[p];
        if (p == pp)
            break; /* Uninited prev value. */

        dist += p < pp ? pp - p : ((ZOPFLI_WINDOW_SIZE - p) + pp);

        chain_counter--;
        if (chain_counter <= 0)
            break;
    }

    StoreInLongestMatchCache(s, pos, limit, sublen, bestdist, bestlength);

    assert(bestlength <= limit);

    *distance = bestdist;
    *length = bestlength;
    assert(pos + *length <= size);
}

/*
Verifies if length and dist are indeed valid, only used for assertion.
*/
void ZopfliVerifyLenDist(const unsigned char *data, size_t datasize, size_t pos,
                         unsigned short dist, unsigned short length)
{

    /* TODO(lode): make this only run in a debug compile, it's for assert only. */
    size_t i;

    assert(pos + length <= datasize);
    for (i = 0; i < length; i++)
    {
        if (data[pos - dist + i] != data[pos + i])
        {
            assert(data[pos - dist + i] == data[pos + i]);
            break;
        }
    }
}

#define HASH_MASK 32767

#define HASH_SHIFT 5

/*
Update the sliding hash value with the given byte. All calls to this function
must be made on consecutive input characters. Since the hash value exists out
of multiple input bytes, a few warmups with this function are needed initially.
*/
static void UpdateHashValue(ZopfliHash *h, unsigned char c)
{
    h->val = (((h->val) << HASH_SHIFT) ^ (c)) & HASH_MASK;
}

/*
Prepopulates hash:
Fills in the initial values in the hash, before ZopfliUpdateHash can be used
correctly.
*/
void ZopfliWarmupHash(const unsigned char *array, size_t pos, size_t end,
                      ZopfliHash *h)
{
    UpdateHashValue(h, array[pos + 0]);
    if (pos + 1 < end)
        UpdateHashValue(h, array[pos + 1]);
}

/*
Appends the length and distance to the LZ77 arrays of the ZopfliLZ77Store.
context must be a ZopfliLZ77Store*.
*/
void ZopfliStoreLitLenDist(unsigned short length, unsigned short dist,
                           size_t pos, ZopfliLZ77Store *store)
{
    size_t i;
    /* Needed for using ZOPFLI_APPEND_DATA multiple times. */
    size_t origsize = store->size;
    size_t llstart = ZOPFLI_NUM_LL * (origsize / ZOPFLI_NUM_LL);
    size_t dstart = ZOPFLI_NUM_D * (origsize / ZOPFLI_NUM_D);

    /* Everytime the index wraps around, a new cumulative histogram is made: we're
    keeping one histogram value per LZ77 symbol rather than a full histogram for
    each to save memory. */
    if (origsize % ZOPFLI_NUM_LL == 0)
    {
        size_t llsize = origsize;
        for (i = 0; i < ZOPFLI_NUM_LL; i++)
        {
            ZOPFLI_APPEND_DATA(
                origsize == 0 ? 0 : store->ll_counts[origsize - ZOPFLI_NUM_LL + i],
                &store->ll_counts, &llsize);
        }
    }
    if (origsize % ZOPFLI_NUM_D == 0)
    {
        size_t dsize = origsize;
        for (i = 0; i < ZOPFLI_NUM_D; i++)
        {
            ZOPFLI_APPEND_DATA(
                origsize == 0 ? 0 : store->d_counts[origsize - ZOPFLI_NUM_D + i],
                &store->d_counts, &dsize);
        }
    }

    ZOPFLI_APPEND_DATA(length, &store->litlens, &store->size);
    store->size = origsize;
    ZOPFLI_APPEND_DATA(dist, &store->dists, &store->size);
    store->size = origsize;
    ZOPFLI_APPEND_DATA(pos, &store->pos, &store->size);
    assert(length < 259);

    if (dist == 0)
    {
        store->size = origsize;
        ZOPFLI_APPEND_DATA(length, &store->ll_symbol, &store->size);
        store->size = origsize;
        ZOPFLI_APPEND_DATA(0, &store->d_symbol, &store->size);
        store->ll_counts[llstart + length]++;
    }
    else
    {
        store->size = origsize;
        ZOPFLI_APPEND_DATA(ZopfliGetLengthSymbol(length),
                           &store->ll_symbol, &store->size);
        store->size = origsize;
        ZOPFLI_APPEND_DATA(ZopfliGetDistSymbol(dist),
                           &store->d_symbol, &store->size);
        store->ll_counts[llstart + ZopfliGetLengthSymbol(length)]++;
        store->d_counts[dstart + ZopfliGetDistSymbol(dist)]++;
    }
}

void ZopfliUpdateHash(const unsigned char *array, size_t pos, size_t end,
                      ZopfliHash *h)
{
    unsigned short hpos = pos & ZOPFLI_WINDOW_MASK;
    size_t amount = 0;

    UpdateHashValue(h, pos + ZOPFLI_MIN_MATCH <= end ? array[pos + ZOPFLI_MIN_MATCH - 1] : 0);
    h->hashval[hpos] = h->val;
    if (h->head[h->val] != -1 && h->hashval[h->head[h->val]] == h->val)
    {
        h->prev[hpos] = h->head[h->val];
    }
    else
        h->prev[hpos] = hpos;
    h->head[h->val] = hpos;

    /* Update "same". */
    if (h->same[(pos - 1) & ZOPFLI_WINDOW_MASK] > 1)
    {
        amount = h->same[(pos - 1) & ZOPFLI_WINDOW_MASK] - 1;
    }
    while (pos + amount + 1 < end &&
           array[pos] == array[pos + amount + 1] && amount < (unsigned short)(-1))
    {
        amount++;
    }
    h->same[hpos] = amount;

    h->val2 = ((h->same[hpos] - ZOPFLI_MIN_MATCH) & 255) ^ h->val;
    h->hashval2[hpos] = h->val2;
    if (h->head2[h->val2] != -1 && h->hashval2[h->head2[h->val2]] == h->val2)
    {
        h->prev2[hpos] = h->head2[h->val2];
    }
    else
        h->prev2[hpos] = hpos;
    h->head2[h->val2] = hpos;
}

void ZopfliResetHash(size_t window_size, ZopfliHash *h)
{
    size_t i;

    h->val = 0;
    for (i = 0; i < 65536; i++)
    {
        h->head[i] = -1; /* -1 indicates no head so far. */
    }
    for (i = 0; i < window_size; i++)
    {
        h->prev[i] = i; /* If prev[j] == j, then prev[j] is uninitialized. */
        h->hashval[i] = -1;
    }

    for (i = 0; i < window_size; i++)
    {
        h->same[i] = 0;
    }

    h->val2 = 0;
    for (i = 0; i < 65536; i++)
    {
        h->head2[i] = -1;
    }
    for (i = 0; i < window_size; i++)
    {
        h->prev2[i] = i;
        h->hashval2[i] = -1;
    }
}

static void FollowPath(ZopfliBlockState *s,
                       const unsigned char *in, size_t instart, size_t inend,
                       unsigned short *path, size_t pathsize,
                       ZopfliLZ77Store *store, ZopfliHash *h)
{
    size_t i, j, pos = 0;
    size_t windowstart = instart > ZOPFLI_WINDOW_SIZE
                             ? instart - ZOPFLI_WINDOW_SIZE
                             : 0;

    size_t total_length_test = 0;

    if (instart == inend)
        return;

    ZopfliResetHash(ZOPFLI_WINDOW_SIZE, h);
    ZopfliWarmupHash(in, windowstart, inend, h);
    for (i = windowstart; i < instart; i++)
    {
        ZopfliUpdateHash(in, i, inend, h);
    }

    pos = instart;
    for (i = 0; i < pathsize; i++)
    {
        unsigned short length = path[i];
        unsigned short dummy_length;
        unsigned short dist;
        assert(pos < inend);

        ZopfliUpdateHash(in, pos, inend, h);

        /* Add to output. */
        if (length >= ZOPFLI_MIN_MATCH)
        {
            /* Get the distance by recalculating longest match. The found length
            should match the length from the path. */
            ZopfliFindLongestMatch(s, h, in, pos, inend, length, 0,
                                   &dist, &dummy_length);
            assert(!(dummy_length != length && length > 2 && dummy_length > 2));
            ZopfliVerifyLenDist(in, inend, pos, dist, length);
            ZopfliStoreLitLenDist(length, dist, pos, store);
            total_length_test += length;
        }
        else
        {
            length = 1;
            ZopfliStoreLitLenDist(in[pos], 0, pos, store);
            total_length_test++;
        }

        assert(pos + length <= inend);
        for (j = 1; j < length; j++)
        {
            ZopfliUpdateHash(in, pos + j, inend, h);
        }

        pos += length;
    }
}

/*
Calculates the optimal path of lz77 lengths to use, from the calculated
length_array. The length_array must contain the optimal length to reach that
byte. The path will be filled with the lengths to use, so its data size will be
the amount of lz77 symbols.
*/
static void TraceBackwards(size_t size, const unsigned short *length_array,
                           unsigned short **path, size_t *pathsize)
{
    size_t index = size;
    if (size == 0)
        return;
    for (;;)
    {
        ZOPFLI_APPEND_DATA(length_array[index], path, pathsize);
        assert(length_array[index] <= index);
        assert(length_array[index] <= ZOPFLI_MAX_MATCH);
        assert(length_array[index] != 0);
        index -= length_array[index];
        if (index == 0)
            break;
    }

    /* Mirror result. */
    for (index = 0; index < *pathsize / 2; index++)
    {
        unsigned short temp = (*path)[index];
        (*path)[index] = (*path)[*pathsize - index - 1];
        (*path)[*pathsize - index - 1] = temp;
    }
}

static size_t zopfli_min(size_t a, size_t b)
{
    return a < b ? a : b;
}

/*
Finds the minimum possible cost this cost model can return for valid length and
distance symbols.
*/
static double GetCostModelMinCost(CostModelFun *costmodel, void *costcontext)
{
    double mincost;
    int bestlength = 0; /* length that has lowest cost in the cost model */
    int bestdist = 0;   /* distance that has lowest cost in the cost model */
    int i;
    /*
    Table of distances that have a different distance symbol in the deflate
    specification. Each value is the first distance that has a new symbol. Only
    different symbols affect the cost model so only these need to be checked.
    See RFC 1951 section 3.2.5. Compressed blocks (length and distance codes).
    */
    static const int dsymbols[30] = {
        1, 2, 3, 4, 5, 7, 9, 13, 17, 25, 33, 49, 65, 97, 129, 193, 257, 385, 513,
        769, 1025, 1537, 2049, 3073, 4097, 6145, 8193, 12289, 16385, 24577};

    mincost = ZOPFLI_LARGE_FLOAT;
    for (i = 3; i < 259; i++)
    {
        double c = costmodel(i, 1, costcontext);
        if (c < mincost)
        {
            bestlength = i;
            mincost = c;
        }
    }

    mincost = ZOPFLI_LARGE_FLOAT;
    for (i = 0; i < 30; i++)
    {
        double c = costmodel(3, dsymbols[i], costcontext);
        if (c < mincost)
        {
            bestdist = dsymbols[i];
            mincost = c;
        }
    }

    return costmodel(bestlength, bestdist, costcontext);
}

/*
Performs the forward pass for "squeeze". Gets the most optimal length to reach
every byte from a previous byte, using cost calculations.
s: the ZopfliBlockState
in: the input data array
instart: where to start
inend: where to stop (not inclusive)
costmodel: function to calculate the cost of some lit/len/dist pair.
costcontext: abstract context for the costmodel function
length_array: output array of size (inend - instart) which will receive the best
length to reach this byte from a previous byte.
returns the cost that was, according to the costmodel, needed to get to the end.
*/
static double GetBestLengths(ZopfliBlockState *s,
                             const unsigned char *in,
                             size_t instart, size_t inend,
                             CostModelFun *costmodel, void *costcontext,
                             unsigned short *length_array,
                             ZopfliHash *h, float *costs)
{
    /* Best cost to get here so far. */
    size_t blocksize = inend - instart;
    size_t i = 0, k, kend;
    unsigned short leng;
    unsigned short dist;
    unsigned short sublen[259];
    size_t windowstart = instart > ZOPFLI_WINDOW_SIZE
                             ? instart - ZOPFLI_WINDOW_SIZE
                             : 0;
    double result;
    double mincost = GetCostModelMinCost(costmodel, costcontext);
    double mincostaddcostj;

    if (instart == inend)
        return 0;

    ZopfliResetHash(ZOPFLI_WINDOW_SIZE, h);
    ZopfliWarmupHash(in, windowstart, inend, h);
    for (i = windowstart; i < instart; i++)
    {
        ZopfliUpdateHash(in, i, inend, h);
    }

    for (i = 1; i < blocksize + 1; i++)
        costs[i] = ZOPFLI_LARGE_FLOAT;
    costs[0] = 0; /* Because it's the start. */
    length_array[0] = 0;

    for (i = instart; i < inend; i++)
    {
        size_t j = i - instart; /* Index in the costs array and length_array. */
        ZopfliUpdateHash(in, i, inend, h);

        /* If we're in a long repetition of the same character and have more than
        ZOPFLI_MAX_MATCH characters before and after our position. */
        if (h->same[i & ZOPFLI_WINDOW_MASK] > ZOPFLI_MAX_MATCH * 2 && i > instart + ZOPFLI_MAX_MATCH + 1 && i + ZOPFLI_MAX_MATCH * 2 + 1 < inend && h->same[(i - ZOPFLI_MAX_MATCH) & ZOPFLI_WINDOW_MASK] > ZOPFLI_MAX_MATCH)
        {
            double symbolcost = costmodel(ZOPFLI_MAX_MATCH, 1, costcontext);
            /* Set the length to reach each one to ZOPFLI_MAX_MATCH, and the cost to
            the cost corresponding to that length. Doing this, we skip
            ZOPFLI_MAX_MATCH values to avoid calling ZopfliFindLongestMatch. */
            for (k = 0; k < ZOPFLI_MAX_MATCH; k++)
            {
                costs[j + ZOPFLI_MAX_MATCH] = costs[j] + symbolcost;
                length_array[j + ZOPFLI_MAX_MATCH] = ZOPFLI_MAX_MATCH;
                i++;
                j++;
                ZopfliUpdateHash(in, i, inend, h);
            }
        }

        ZopfliFindLongestMatch(s, h, in, i, inend, ZOPFLI_MAX_MATCH, sublen,
                               &dist, &leng);

        /* Literal. */
        if (i + 1 <= inend)
        {
            double newCost = costmodel(in[i], 0, costcontext) + costs[j];
            assert(newCost >= 0);
            if (newCost < costs[j + 1])
            {
                costs[j + 1] = newCost;
                length_array[j + 1] = 1;
            }
        }
        /* Lengths. */
        kend = zopfli_min(leng, inend - i);
        mincostaddcostj = mincost + costs[j];
        for (k = 3; k <= kend; k++)
        {
            double newCost;

            /* Calling the cost model is expensive, avoid this if we are already at
            the minimum possible cost that it can return. */
            if (costs[j + k] <= mincostaddcostj)
                continue;

            newCost = costmodel(k, sublen[k], costcontext) + costs[j];
            assert(newCost >= 0);
            if (newCost < costs[j + k])
            {
                assert(k <= ZOPFLI_MAX_MATCH);
                costs[j + k] = newCost;
                length_array[j + k] = k;
            }
        }
    }

    assert(costs[blocksize] >= 0);
    result = costs[blocksize];

    return result;
}

/*
Does a single run for ZopfliLZ77Optimal. For good compression, repeated runs
with updated statistics should be performed.
s: the block state
in: the input data array
instart: where to start
inend: where to stop (not inclusive)
path: pointer to dynamically allocated memory to store the path
pathsize: pointer to the size of the dynamic path array
length_array: array of size (inend - instart) used to store lengths
costmodel: function to use as the cost model for this squeeze run
costcontext: abstract context for the costmodel function
store: place to output the LZ77 data
returns the cost that was, according to the costmodel, needed to get to the end.
This is not the actual cost.
*/
static double LZ77OptimalRun(ZopfliBlockState *s,
                             const unsigned char *in, size_t instart, size_t inend,
                             unsigned short **path, size_t *pathsize,
                             unsigned short *length_array, CostModelFun *costmodel,
                             void *costcontext, ZopfliLZ77Store *store,
                             ZopfliHash *h, float *costs)
{
    double cost = GetBestLengths(s, in, instart, inend, costmodel,
                                 costcontext, length_array, h, costs);
    free(*path);
    *path = 0;
    *pathsize = 0;
    TraceBackwards(inend - instart, length_array, path, pathsize);
    FollowPath(s, in, instart, inend, *path, *pathsize, store, h);
    assert(cost < ZOPFLI_LARGE_FLOAT);
    return cost;
}

void ZopfliCleanHash(ZopfliHash *h)
{
    free(h->head);
    free(h->prev);
    free(h->hashval);

    free(h->head2);
    free(h->prev2);
    free(h->hashval2);

    free(h->same);
}

void ZopfliAllocHash(size_t window_size, ZopfliHash *h)
{
    h->head = (int *)malloc(sizeof(*h->head) * 65536);
    h->prev = (unsigned short *)malloc(sizeof(*h->prev) * window_size);
    h->hashval = (int *)malloc(sizeof(*h->hashval) * window_size);

    h->same = (unsigned short *)malloc(sizeof(*h->same) * window_size);

    h->head2 = (int *)malloc(sizeof(*h->head2) * 65536);
    h->prev2 = (unsigned short *)malloc(sizeof(*h->prev2) * window_size);
    h->hashval2 = (int *)malloc(sizeof(*h->hashval2) * window_size);
}

/*
Does the same as ZopfliLZ77Optimal, but optimized for the fixed tree of the
deflate standard.
The fixed tree never gives the best compression. But this gives the best
possible LZ77 encoding possible with the fixed tree.
This does not create or output any fixed tree, only LZ77 data optimized for
using with a fixed tree.
If instart is larger than 0, it uses values before instart as starting
dictionary.
*/
void ZopfliLZ77OptimalFixed(ZopfliBlockState *s,
                            const unsigned char *in,
                            size_t instart, size_t inend,
                            ZopfliLZ77Store *store)
{
    /* Dist to get to here with smallest cost. */
    size_t blocksize = inend - instart;
    unsigned short *length_array =
        (unsigned short *)malloc(sizeof(unsigned short) * (blocksize + 1));
    unsigned short *path = 0;
    size_t pathsize = 0;
    ZopfliHash hash;
    ZopfliHash *h = &hash;
    float *costs = (float *)malloc(sizeof(float) * (blocksize + 1));

    if (!costs)
        exit(-1); /* Allocation failed. */
    if (!length_array)
        exit(-1); /* Allocation failed. */

    ZopfliAllocHash(ZOPFLI_WINDOW_SIZE, h);

    s->blockstart = instart;
    s->blockend = inend;

    /* Shortest path for fixed tree This one should give the shortest possible
    result for fixed tree, no repeated runs are needed since the tree is known. */
    LZ77OptimalRun(s, in, instart, inend, &path, &pathsize,
                   length_array, GetCostFixed, 0, store, h, costs);

    free(length_array);
    free(path);
    free(costs);
    ZopfliCleanHash(h);
}

void ZopfliCleanCache(ZopfliLongestMatchCache *lmc)
{
    free(lmc->length);
    free(lmc->dist);
    free(lmc->sublen);
}

void ZopfliCleanBlockState(ZopfliBlockState *s)
{
    if (s->lmc)
    {
        ZopfliCleanCache(s->lmc);
        free(s->lmc);
    }
}

void ZopfliInitCache(size_t blocksize, ZopfliLongestMatchCache *lmc)
{
    size_t i;
    lmc->length = (unsigned short *)malloc(sizeof(unsigned short) * blocksize);
    lmc->dist = (unsigned short *)malloc(sizeof(unsigned short) * blocksize);
    /* Rather large amount of memory. */
    lmc->sublen = (unsigned char *)malloc(ZOPFLI_CACHE_LENGTH * 3 * blocksize);
    if (lmc->sublen == NULL)
    {
        fprintf(stderr,
                "Error: Out of memory. Tried allocating %lu bytes of memory.\n",
                (unsigned long)ZOPFLI_CACHE_LENGTH * 3 * blocksize);
        exit(EXIT_FAILURE);
    }

    /* length > 0 and dist 0 is invalid combination, which indicates on purpose
    that this cache value is not filled in yet. */
    for (i = 0; i < blocksize; i++)
        lmc->length[i] = 1;
    for (i = 0; i < blocksize; i++)
        lmc->dist[i] = 0;
    for (i = 0; i < ZOPFLI_CACHE_LENGTH * blocksize * 3; i++)
        lmc->sublen[i] = 0;
}

void ZopfliInitBlockState(const ZopfliOptions *options,
                          size_t blockstart, size_t blockend, int add_lmc,
                          ZopfliBlockState *s)
{
    s->options = options;
    s->blockstart = blockstart;
    s->blockend = blockend;
    if (add_lmc)
    {
        s->lmc = (ZopfliLongestMatchCache *)malloc(sizeof(ZopfliLongestMatchCache));
        ZopfliInitCache(blockend - blockstart, s->lmc);
    }
    else
    {
        s->lmc = 0;
    }
}

void ZopfliCleanLZ77Store(ZopfliLZ77Store *store)
{
    free(store->litlens);
    free(store->dists);
    free(store->pos);
    free(store->ll_symbol);
    free(store->d_symbol);
    free(store->ll_counts);
    free(store->d_counts);
}

void ZopfliInitLZ77Store(const unsigned char *data, ZopfliLZ77Store *store)
{
    store->size = 0;
    store->litlens = 0;
    store->dists = 0;
    store->pos = 0;
    store->data = data;
    store->ll_symbol = 0;
    store->d_symbol = 0;
    store->ll_counts = 0;
    store->d_counts = 0;
}

static void AddLZ77BlockAutoType(const ZopfliOptions *options, int final,
                                 const ZopfliLZ77Store *lz77,
                                 size_t lstart, size_t lend,
                                 size_t expected_data_size,
                                 unsigned char *bp,
                                 unsigned char **out, size_t *outsize)
{
    double uncompressedcost = ZopfliCalculateBlockSize(lz77, lstart, lend, 0);
    double fixedcost = ZopfliCalculateBlockSize(lz77, lstart, lend, 1);
    double dyncost = ZopfliCalculateBlockSize(lz77, lstart, lend, 2);

    /* Whether to perform the expensive calculation of creating an optimal block
    with fixed huffman tree to check if smaller. Only do this for small blocks or
    blocks which already are pretty good with fixed huffman tree. */
    int expensivefixed = (lz77->size < 1000) || fixedcost <= dyncost * 1.1;

    ZopfliLZ77Store fixedstore;
    if (lstart == lend)
    {
        /* Smallest empty block is represented by fixed block */
        AddBits(final, 1, bp, out, outsize);
        AddBits(1, 2, bp, out, outsize); /* btype 01 */
        AddBits(0, 7, bp, out, outsize); /* end symbol has code 0000000 */
        return;
    }
    ZopfliInitLZ77Store(lz77->data, &fixedstore);
    if (expensivefixed)
    {
        /* Recalculate the LZ77 with ZopfliLZ77OptimalFixed */
        size_t instart = lz77->pos[lstart];
        size_t inend = instart + ZopfliLZ77GetByteRange(lz77, lstart, lend);

        ZopfliBlockState s;
        ZopfliInitBlockState(options, instart, inend, 1, &s);
        ZopfliLZ77OptimalFixed(&s, lz77->data, instart, inend, &fixedstore);
        fixedcost = ZopfliCalculateBlockSize(&fixedstore, 0, fixedstore.size, 1);
        ZopfliCleanBlockState(&s);
    }

    if (uncompressedcost < fixedcost && uncompressedcost < dyncost)
    {
        AddLZ77Block(options, 0, final, lz77, lstart, lend,
                     expected_data_size, bp, out, outsize);
    }
    else if (fixedcost < dyncost)
    {
        if (expensivefixed)
        {
            AddLZ77Block(options, 1, final, &fixedstore, 0, fixedstore.size,
                         expected_data_size, bp, out, outsize);
        }
        else
        {
            AddLZ77Block(options, 1, final, lz77, lstart, lend,
                         expected_data_size, bp, out, outsize);
        }
    }
    else
    {
        AddLZ77Block(options, 2, final, lz77, lstart, lend,
                     expected_data_size, bp, out, outsize);
    }

    ZopfliCleanLZ77Store(&fixedstore);
}

/*
Gets a score of the length given the distance. Typically, the score of the
length is the length itself, but if the distance is very long, decrease the
score of the length a bit to make up for the fact that long distances use large
amounts of extra bits.

This is not an accurate score, it is a heuristic only for the greedy LZ77
implementation. More accurate cost models are employed later. Making this
heuristic more accurate may hurt rather than improve compression.

The two direct uses of this heuristic are:
-avoid using a length of 3 in combination with a long distance. This only has
an effect if length == 3.
-make a slightly better choice between the two options of the lazy matching.

Indirectly, this affects:
-the block split points if the default of block splitting first is used, in a
rather unpredictable way
-the first zopfli run, so it affects the chance of the first run being closer
to the optimal output
*/
static int GetLengthScore(int length, int distance)
{
    /*
    At 1024, the distance uses 9+ extra bits and this seems to be the sweet spot
    on tested files.
    */
    return distance > 1024 ? length - 1 : length;
}

/*
Does LZ77 using an algorithm similar to gzip, with lazy matching, rather than
with the slow but better "squeeze" implementation.
The result is placed in the ZopfliLZ77Store.
If instart is larger than 0, it uses values before instart as starting
dictionary.
*/
void ZopfliLZ77Greedy(ZopfliBlockState *s, const unsigned char *in,
                      size_t instart, size_t inend,
                      ZopfliLZ77Store *store, ZopfliHash *h)
{
    size_t i = 0, j;
    unsigned short leng;
    unsigned short dist;
    int lengthscore;
    size_t windowstart = instart > ZOPFLI_WINDOW_SIZE
                             ? instart - ZOPFLI_WINDOW_SIZE
                             : 0;
    unsigned short dummysublen[259];

    /* Lazy matching. */
    unsigned prev_length = 0;
    unsigned prev_match = 0;
    int prevlengthscore;
    int match_available = 0;

    if (instart == inend)
        return;

    ZopfliResetHash(ZOPFLI_WINDOW_SIZE, h);
    ZopfliWarmupHash(in, windowstart, inend, h);
    for (i = windowstart; i < instart; i++)
    {
        ZopfliUpdateHash(in, i, inend, h);
    }

    for (i = instart; i < inend; i++)
    {
        ZopfliUpdateHash(in, i, inend, h);

        ZopfliFindLongestMatch(s, h, in, i, inend, ZOPFLI_MAX_MATCH, dummysublen,
                               &dist, &leng);
        lengthscore = GetLengthScore(leng, dist);

        /* Lazy matching. */
        prevlengthscore = GetLengthScore(prev_length, prev_match);
        if (match_available)
        {
            match_available = 0;
            if (lengthscore > prevlengthscore + 1)
            {
                ZopfliStoreLitLenDist(in[i - 1], 0, i - 1, store);
                if (lengthscore >= ZOPFLI_MIN_MATCH && leng < ZOPFLI_MAX_MATCH)
                {
                    match_available = 1;
                    prev_length = leng;
                    prev_match = dist;
                    continue;
                }
            }
            else
            {
                /* Add previous to output. */
                leng = prev_length;
                dist = prev_match;
                lengthscore = prevlengthscore;
                /* Add to output. */
                ZopfliVerifyLenDist(in, inend, i - 1, dist, leng);
                ZopfliStoreLitLenDist(leng, dist, i - 1, store);
                for (j = 2; j < leng; j++)
                {
                    assert(i < inend);
                    i++;
                    ZopfliUpdateHash(in, i, inend, h);
                }
                continue;
            }
        }
        else if (lengthscore >= ZOPFLI_MIN_MATCH && leng < ZOPFLI_MAX_MATCH)
        {
            match_available = 1;
            prev_length = leng;
            prev_match = dist;
            continue;
        }
        /* End of lazy matching. */

        /* Add to output. */
        if (lengthscore >= ZOPFLI_MIN_MATCH)
        {
            ZopfliVerifyLenDist(in, inend, i, dist, leng);
            ZopfliStoreLitLenDist(leng, dist, i, store);
        }
        else
        {
            leng = 1;
            ZopfliStoreLitLenDist(in[i], 0, i, store);
        }
        for (j = 1; j < leng; j++)
        {
            assert(i < inend);
            i++;
            ZopfliUpdateHash(in, i, inend, h);
        }
    }
}

/*
Calculates the entropy of each symbol, based on the counts of each symbol. The
result is similar to the result of ZopfliCalculateBitLengths, but with the
actual theoritical bit lengths according to the entropy. Since the resulting
values are fractional, they cannot be used to encode the tree specified by
DEFLATE.
*/
void ZopfliCalculateEntropy(const size_t *count, size_t n, double *bitlengths)
{
    static const double kInvLog2 = 1.4426950408889; /* 1.0 / log(2.0) */
    unsigned sum = 0;
    unsigned i;
    double log2sum;
    for (i = 0; i < n; ++i)
    {
        sum += count[i];
    }
    log2sum = (sum == 0 ? log(n) : log(sum)) * kInvLog2;
    for (i = 0; i < n; ++i)
    {
        /* When the count of the symbol is 0, but its cost is requested anyway, it
        means the symbol will appear at least once anyway, so give it the cost as if
        its count is 1.*/
        if (count[i] == 0)
            bitlengths[i] = log2sum;
        else
            bitlengths[i] = log2sum - log(count[i]) * kInvLog2;
        /* Depending on compiler and architecture, the above subtraction of two
        floating point numbers may give a negative result very close to zero
        instead of zero (e.g. -5.973954e-17 with gcc 4.1.2 on Ubuntu 11.4). Clamp
        it to zero. These floating point imprecisions do not affect the cost model
        significantly so this is ok. */
        if (bitlengths[i] < 0 && bitlengths[i] > -1e-5)
            bitlengths[i] = 0;
        assert(bitlengths[i] >= 0);
    }
}

/* Calculates the entropy of the statistics */
static void CalculateStatistics(SymbolStats *stats)
{
    ZopfliCalculateEntropy(stats->litlens, ZOPFLI_NUM_LL, stats->ll_symbols);
    ZopfliCalculateEntropy(stats->dists, ZOPFLI_NUM_D, stats->d_symbols);
}

/* Appends the symbol statistics from the store. */
static void GetStatistics(const ZopfliLZ77Store *store, SymbolStats *stats)
{
    size_t i;
    for (i = 0; i < store->size; i++)
    {
        if (store->dists[i] == 0)
        {
            stats->litlens[store->litlens[i]]++;
        }
        else
        {
            stats->litlens[ZopfliGetLengthSymbol(store->litlens[i])]++;
            stats->dists[ZopfliGetDistSymbol(store->dists[i])]++;
        }
    }
    stats->litlens[256] = 1; /* End symbol. */

    CalculateStatistics(stats);
}

static void ClearStatFreqs(SymbolStats *stats)
{
    size_t i;
    for (i = 0; i < ZOPFLI_NUM_LL; i++)
        stats->litlens[i] = 0;
    for (i = 0; i < ZOPFLI_NUM_D; i++)
        stats->dists[i] = 0;
}

/* Get random number: "Multiply-With-Carry" generator of G. Marsaglia */
static unsigned int Ran(RanState *state)
{
    state->m_z = 36969 * (state->m_z & 65535) + (state->m_z >> 16);
    state->m_w = 18000 * (state->m_w & 65535) + (state->m_w >> 16);
    return (state->m_z << 16) + state->m_w; /* 32-bit result. */
}

static void RandomizeFreqs(RanState *state, size_t *freqs, int n)
{
    int i;
    for (i = 0; i < n; i++)
    {
        if ((Ran(state) >> 4) % 3 == 0)
            freqs[i] = freqs[Ran(state) % n];
    }
}

static void RandomizeStatFreqs(RanState *state, SymbolStats *stats)
{
    RandomizeFreqs(state, stats->litlens, ZOPFLI_NUM_LL);
    RandomizeFreqs(state, stats->dists, ZOPFLI_NUM_D);
    stats->litlens[256] = 1; /* End symbol. */
}

static void InitRanState(RanState *state)
{
    state->m_w = 1;
    state->m_z = 2;
}

/* Adds the bit lengths. */
static void AddWeighedStatFreqs(const SymbolStats *stats1, double w1,
                                const SymbolStats *stats2, double w2,
                                SymbolStats *result)
{
    size_t i;
    for (i = 0; i < ZOPFLI_NUM_LL; i++)
    {
        result->litlens[i] =
            (size_t)(stats1->litlens[i] * w1 + stats2->litlens[i] * w2);
    }
    for (i = 0; i < ZOPFLI_NUM_D; i++)
    {
        result->dists[i] =
            (size_t)(stats1->dists[i] * w1 + stats2->dists[i] * w2);
    }
    result->litlens[256] = 1; /* End symbol. */
}

static size_t CeilDiv(size_t a, size_t b)
{
    return (a + b - 1) / b;
}

void ZopfliCopyLZ77Store(
    const ZopfliLZ77Store *source, ZopfliLZ77Store *dest)
{
    size_t i;
    size_t llsize = ZOPFLI_NUM_LL * CeilDiv(source->size, ZOPFLI_NUM_LL);
    size_t dsize = ZOPFLI_NUM_D * CeilDiv(source->size, ZOPFLI_NUM_D);
    ZopfliCleanLZ77Store(dest);
    ZopfliInitLZ77Store(source->data, dest);
    dest->litlens =
        (unsigned short *)malloc(sizeof(*dest->litlens) * source->size);
    dest->dists = (unsigned short *)malloc(sizeof(*dest->dists) * source->size);
    dest->pos = (size_t *)malloc(sizeof(*dest->pos) * source->size);
    dest->ll_symbol =
        (unsigned short *)malloc(sizeof(*dest->ll_symbol) * source->size);
    dest->d_symbol =
        (unsigned short *)malloc(sizeof(*dest->d_symbol) * source->size);
    dest->ll_counts = (size_t *)malloc(sizeof(*dest->ll_counts) * llsize);
    dest->d_counts = (size_t *)malloc(sizeof(*dest->d_counts) * dsize);

    /* Allocation failed. */
    if (!dest->litlens || !dest->dists)
        exit(-1);
    if (!dest->pos)
        exit(-1);
    if (!dest->ll_symbol || !dest->d_symbol)
        exit(-1);
    if (!dest->ll_counts || !dest->d_counts)
        exit(-1);

    dest->size = source->size;
    for (i = 0; i < source->size; i++)
    {
        dest->litlens[i] = source->litlens[i];
        dest->dists[i] = source->dists[i];
        dest->pos[i] = source->pos[i];
        dest->ll_symbol[i] = source->ll_symbol[i];
        dest->d_symbol[i] = source->d_symbol[i];
    }
    for (i = 0; i < llsize; i++)
    {
        dest->ll_counts[i] = source->ll_counts[i];
    }
    for (i = 0; i < dsize; i++)
    {
        dest->d_counts[i] = source->d_counts[i];
    }
}

static void CopyStats(SymbolStats *source, SymbolStats *dest)
{
    memcpy(dest->litlens, source->litlens,
           ZOPFLI_NUM_LL * sizeof(dest->litlens[0]));
    memcpy(dest->dists, source->dists, ZOPFLI_NUM_D * sizeof(dest->dists[0]));

    memcpy(dest->ll_symbols, source->ll_symbols,
           ZOPFLI_NUM_LL * sizeof(dest->ll_symbols[0]));
    memcpy(dest->d_symbols, source->d_symbols,
           ZOPFLI_NUM_D * sizeof(dest->d_symbols[0]));
}

/* Sets everything to 0. */
static void InitStats(SymbolStats *stats)
{
    memset(stats->litlens, 0, ZOPFLI_NUM_LL * sizeof(stats->litlens[0]));
    memset(stats->dists, 0, ZOPFLI_NUM_D * sizeof(stats->dists[0]));

    memset(stats->ll_symbols, 0, ZOPFLI_NUM_LL * sizeof(stats->ll_symbols[0]));
    memset(stats->d_symbols, 0, ZOPFLI_NUM_D * sizeof(stats->d_symbols[0]));
}

/*
Calculates lit/len and dist pairs for given data.
If instart is larger than 0, it uses values before instart as starting
dictionary.
*/
void ZopfliLZ77Optimal(ZopfliBlockState *s,
                       const unsigned char *in, size_t instart, size_t inend,
                       int numiterations,
                       ZopfliLZ77Store *store)
{
    /* Dist to get to here with smallest cost. */
    size_t blocksize = inend - instart;
    unsigned short *length_array =
        (unsigned short *)malloc(sizeof(unsigned short) * (blocksize + 1));
    unsigned short *path = 0;
    size_t pathsize = 0;
    ZopfliLZ77Store currentstore;
    ZopfliHash hash;
    ZopfliHash *h = &hash;
    SymbolStats stats, beststats, laststats;
    int i;
    float *costs = (float *)malloc(sizeof(float) * (blocksize + 1));
    double cost;
    double bestcost = ZOPFLI_LARGE_FLOAT;
    double lastcost = 0;
    /* Try randomizing the costs a bit once the size stabilizes. */
    RanState ran_state;
    int lastrandomstep = -1;

    if (!costs)
        exit(-1); /* Allocation failed. */
    if (!length_array)
        exit(-1); /* Allocation failed. */

    InitRanState(&ran_state);
    InitStats(&stats);
    ZopfliInitLZ77Store(in, &currentstore);
    ZopfliAllocHash(ZOPFLI_WINDOW_SIZE, h);

    /* Do regular deflate, then loop multiple shortest path runs, each time using
    the statistics of the previous run. */

    /* Initial run. */
    ZopfliLZ77Greedy(s, in, instart, inend, &currentstore, h);
    GetStatistics(&currentstore, &stats);

    /* Repeat statistics with each time the cost model from the previous stat
    run. */
    for (i = 0; i < numiterations; i++)
    {
        ZopfliCleanLZ77Store(&currentstore);
        ZopfliInitLZ77Store(in, &currentstore);
        LZ77OptimalRun(s, in, instart, inend, &path, &pathsize,
                       length_array, GetCostStat, (void *)&stats,
                       &currentstore, h, costs);
        cost = ZopfliCalculateBlockSize(&currentstore, 0, currentstore.size, 2);
        if (s->options->verbose_more || (s->options->verbose && cost < bestcost))
        {
            fprintf(stderr, "Iteration %d: %d bit\n", i, (int)cost);
        }
        if (cost < bestcost)
        {
            /* Copy to the output store. */
            ZopfliCopyLZ77Store(&currentstore, store);
            CopyStats(&stats, &beststats);
            bestcost = cost;
        }
        CopyStats(&stats, &laststats);
        ClearStatFreqs(&stats);
        GetStatistics(&currentstore, &stats);
        if (lastrandomstep != -1)
        {
            /* This makes it converge slower but better. Do it only once the
            randomness kicks in so that if the user does few iterations, it gives a
            better result sooner. */
            AddWeighedStatFreqs(&stats, 1.0, &laststats, 0.5, &stats);
            CalculateStatistics(&stats);
        }
        if (i > 5 && cost == lastcost)
        {
            CopyStats(&beststats, &stats);
            RandomizeStatFreqs(&ran_state, &stats);
            CalculateStatistics(&stats);
            lastrandomstep = i;
        }
        lastcost = cost;
    }

    free(length_array);
    free(path);
    free(costs);
    ZopfliCleanLZ77Store(&currentstore);
    ZopfliCleanHash(h);
}

/*
Finds next block to try to split, the largest of the available ones.
The largest is chosen to make sure that if only a limited amount of blocks is
requested, their sizes are spread evenly.
lz77size: the size of the LL77 data, which is the size of the done array here.
done: array indicating which blocks starting at that position are no longer
splittable (splitting them increases rather than decreases cost).
splitpoints: the splitpoints found so far.
npoints: the amount of splitpoints found so far.
lstart: output variable, giving start of block.
lend: output variable, giving end of block.
returns 1 if a block was found, 0 if no block found (all are done).
*/
static int FindLargestSplittableBlock(
    size_t lz77size, const unsigned char *done,
    const size_t *splitpoints, size_t npoints,
    size_t *lstart, size_t *lend)
{
    size_t longest = 0;
    int found = 0;
    size_t i;
    for (i = 0; i <= npoints; i++)
    {
        size_t start = i == 0 ? 0 : splitpoints[i - 1];
        size_t end = i == npoints ? lz77size - 1 : splitpoints[i];
        if (!done[start] && end - start > longest)
        {
            *lstart = start;
            *lend = end;
            found = 1;
            longest = end - start;
        }
    }
    return found;
}

/*
Prints the block split points as decimal and hex values in the terminal.
*/
static void PrintBlockSplitPoints(const ZopfliLZ77Store *lz77,
                                  const size_t *lz77splitpoints,
                                  size_t nlz77points)
{
    size_t *splitpoints = 0;
    size_t npoints = 0;
    size_t i;
    /* The input is given as lz77 indices, but we want to see the uncompressed
    index values. */
    size_t pos = 0;
    if (nlz77points > 0)
    {
        for (i = 0; i < lz77->size; i++)
        {
            size_t length = lz77->dists[i] == 0 ? 1 : lz77->litlens[i];
            if (lz77splitpoints[npoints] == i)
            {
                ZOPFLI_APPEND_DATA(pos, &splitpoints, &npoints);
                if (npoints == nlz77points)
                    break;
            }
            pos += length;
        }
    }
    assert(npoints == nlz77points);

    fprintf(stderr, "block split points: ");
    for (i = 0; i < npoints; i++)
    {
        fprintf(stderr, "%d ", (int)splitpoints[i]);
    }
    fprintf(stderr, "(hex:");
    for (i = 0; i < npoints; i++)
    {
        fprintf(stderr, " %x", (int)splitpoints[i]);
    }
    fprintf(stderr, ")\n");

    free(splitpoints);
}

static void AddSorted(size_t value, size_t **out, size_t *outsize)
{
    size_t i;
    ZOPFLI_APPEND_DATA(value, out, outsize);
    for (i = 0; i + 1 < *outsize; i++)
    {
        if ((*out)[i] > value)
        {
            size_t j;
            for (j = *outsize - 1; j > i; j--)
            {
                (*out)[j] = (*out)[j - 1];
            }
            (*out)[i] = value;
            break;
        }
    }
}

/* Try to find minimum faster by recursively checking multiple points. */
#define NUM 9 /* Good value: 9. */

/*
Finds minimum of function f(i) where is is of type size_t, f(i) is of type
double, i is in range start-end (excluding end).
Outputs the minimum value in *smallest and returns the index of this value.
*/
static size_t FindMinimum(FindMinimumFun f, void *context,
                          size_t start, size_t end, double *smallest)
{
    if (end - start < 1024)
    {
        double best = ZOPFLI_LARGE_FLOAT;
        size_t result = start;
        size_t i;
        for (i = start; i < end; i++)
        {
            double v = f(i, context);
            if (v < best)
            {
                best = v;
                result = i;
            }
        }
        *smallest = best;
        return result;
    }
    else
    {
        /* Try to find minimum faster by recursively checking multiple points. */
#define NUM 9 /* Good value: 9. */
        size_t i;
        size_t p[NUM];
        double vp[NUM];
        size_t besti;
        double best;
        double lastbest = ZOPFLI_LARGE_FLOAT;
        size_t pos = start;

        for (;;)
        {
            if (end - start <= NUM)
                break;

            for (i = 0; i < NUM; i++)
            {
                p[i] = start + (i + 1) * ((end - start) / (NUM + 1));
                vp[i] = f(p[i], context);
            }
            besti = 0;
            best = vp[0];
            for (i = 1; i < NUM; i++)
            {
                if (vp[i] < best)
                {
                    best = vp[i];
                    besti = i;
                }
            }
            if (best > lastbest)
                break;

            start = besti == 0 ? start : p[besti - 1];
            end = besti == NUM - 1 ? end : p[besti + 1];

            pos = p[besti];
            lastbest = best;
        }
        *smallest = lastbest;
        return pos;
#undef NUM
    }
}

/*
Does blocksplitting on LZ77 data.
The output splitpoints are indices in the LZ77 data.
maxblocks: set a limit to the amount of blocks. Set to 0 to mean no limit.
*/
void ZopfliBlockSplitLZ77(const ZopfliOptions *options,
                          const ZopfliLZ77Store *lz77, size_t maxblocks,
                          size_t **splitpoints, size_t *npoints)
{
    size_t lstart, lend;
    size_t i;
    size_t llpos = 0;
    size_t numblocks = 1;
    unsigned char *done;
    double splitcost, origcost;

    if (lz77->size < 10)
        return; /* This code fails on tiny files. */

    done = (unsigned char *)malloc(lz77->size);
    if (!done)
        exit(-1); /* Allocation failed. */
    for (i = 0; i < lz77->size; i++)
        done[i] = 0;

    lstart = 0;
    lend = lz77->size;
    for (;;)
    {
        SplitCostContext c;

        if (maxblocks > 0 && numblocks >= maxblocks)
        {
            break;
        }

        c.lz77 = lz77;
        c.start = lstart;
        c.end = lend;
        assert(lstart < lend);
        llpos = FindMinimum(SplitCost, &c, lstart + 1, lend, &splitcost);

        assert(llpos > lstart);
        assert(llpos < lend);

        origcost = EstimateCost(lz77, lstart, lend);

        if (splitcost > origcost || llpos == lstart + 1 || llpos == lend)
        {
            done[lstart] = 1;
        }
        else
        {
            AddSorted(llpos, splitpoints, npoints);
            numblocks++;
        }

        if (!FindLargestSplittableBlock(
                lz77->size, done, *splitpoints, *npoints, &lstart, &lend))
        {
            break; /* No further split will probably reduce compression. */
        }

        if (lend - lstart < 10)
        {
            break;
        }
    }

    if (options->verbose)
    {
        PrintBlockSplitPoints(lz77, *splitpoints, *npoints);
    }

    free(done);
}

/*
Does blocksplitting on uncompressed data.
The output splitpoints are indices in the uncompressed bytes.

options: general program options.
in: uncompressed input data
instart: where to start splitting
inend: where to end splitting (not inclusive)
maxblocks: maximum amount of blocks to split into, or 0 for no limit
splitpoints: dynamic array to put the resulting split point coordinates into.
The coordinates are indices in the input array.
npoints: pointer to amount of splitpoints, for the dynamic array. The amount of
blocks is the amount of splitpoitns + 1.
*/
void ZopfliBlockSplit(const ZopfliOptions *options,
                      const unsigned char *in, size_t instart, size_t inend,
                      size_t maxblocks, size_t **splitpoints, size_t *npoints)
{
    size_t pos = 0;
    size_t i;
    ZopfliBlockState s;
    size_t *lz77splitpoints = 0;
    size_t nlz77points = 0;
    ZopfliLZ77Store store;
    ZopfliHash hash;
    ZopfliHash *h = &hash;

    ZopfliInitLZ77Store(in, &store);
    ZopfliInitBlockState(options, instart, inend, 0, &s);
    ZopfliAllocHash(ZOPFLI_WINDOW_SIZE, h);

    *npoints = 0;
    *splitpoints = 0;

    /* Unintuitively, Using a simple LZ77 method here instead of ZopfliLZ77Optimal
    results in better blocks. */
    ZopfliLZ77Greedy(&s, in, instart, inend, &store, h);

    ZopfliBlockSplitLZ77(options,
                         &store, maxblocks,
                         &lz77splitpoints, &nlz77points);

    /* Convert LZ77 positions to positions in the uncompressed input. */
    pos = instart;
    if (nlz77points > 0)
    {
        for (i = 0; i < store.size; i++)
        {
            size_t length = store.dists[i] == 0 ? 1 : store.litlens[i];
            if (lz77splitpoints[*npoints] == i)
            {
                ZOPFLI_APPEND_DATA(pos, splitpoints, npoints);
                if (*npoints == nlz77points)
                    break;
            }
            pos += length;
        }
    }
    assert(*npoints == nlz77points);

    free(lz77splitpoints);
    ZopfliCleanBlockState(&s);
    ZopfliCleanLZ77Store(&store);
    ZopfliCleanHash(h);
}

void ZopfliAppendLZ77Store(const ZopfliLZ77Store *store,
                           ZopfliLZ77Store *target)
{
    size_t i;
    for (i = 0; i < store->size; i++)
    {
        ZopfliStoreLitLenDist(store->litlens[i], store->dists[i],
                              store->pos[i], target);
    }
}

/*
Deflate a part, to allow ZopfliDeflate() to use multiple master blocks if
needed.
It is possible to call this function multiple times in a row, shifting
instart and inend to next bytes of the data. If instart is larger than 0, then
previous bytes are used as the initial dictionary for LZ77.
This function will usually output multiple deflate blocks. If final is 1, then
the final bit will be set on the last block.
*/
void ZopfliDeflatePart(const ZopfliOptions *options, int btype, int final,
                       const unsigned char *in, size_t instart, size_t inend,
                       unsigned char *bp, unsigned char **out,
                       size_t *outsize)
{
    size_t i;
    /* byte coordinates rather than lz77 index */
    size_t *splitpoints_uncompressed = 0;
    size_t npoints = 0;
    size_t *splitpoints = 0;
    double totalcost = 0;
    ZopfliLZ77Store lz77;

    /* If btype=2 is specified, it tries all block types. If a lesser btype is
    given, then however it forces that one. Neither of the lesser types needs
    block splitting as they have no dynamic huffman trees. */
    if (btype == 0)
    {
        AddNonCompressedBlock(options, final, in, instart, inend, bp, out, outsize);
        return;
    }
    else if (btype == 1)
    {
        ZopfliLZ77Store store;
        ZopfliBlockState s;
        ZopfliInitLZ77Store(in, &store);
        ZopfliInitBlockState(options, instart, inend, 1, &s);

        ZopfliLZ77OptimalFixed(&s, in, instart, inend, &store);
        AddLZ77Block(options, btype, final, &store, 0, store.size, 0,
                     bp, out, outsize);

        ZopfliCleanBlockState(&s);
        ZopfliCleanLZ77Store(&store);
        return;
    }

    if (options->blocksplitting)
    {
        ZopfliBlockSplit(options, in, instart, inend,
                         options->blocksplittingmax,
                         &splitpoints_uncompressed, &npoints);
        splitpoints = (size_t *)malloc(sizeof(*splitpoints) * npoints);
    }

    ZopfliInitLZ77Store(in, &lz77);

    for (i = 0; i <= npoints; i++)
    {
        size_t start = i == 0 ? instart : splitpoints_uncompressed[i - 1];
        size_t end = i == npoints ? inend : splitpoints_uncompressed[i];
        ZopfliBlockState s;
        ZopfliLZ77Store store;
        ZopfliInitLZ77Store(in, &store);
        ZopfliInitBlockState(options, start, end, 1, &s);
        ZopfliLZ77Optimal(&s, in, start, end, options->numiterations, &store);
        totalcost += ZopfliCalculateBlockSizeAutoType(&store, 0, store.size);

        ZopfliAppendLZ77Store(&store, &lz77);
        if (i < npoints)
            splitpoints[i] = lz77.size;

        ZopfliCleanBlockState(&s);
        ZopfliCleanLZ77Store(&store);
    }

    /* Second block splitting attempt */
    if (options->blocksplitting && npoints > 1)
    {
        size_t *splitpoints2 = 0;
        size_t npoints2 = 0;
        double totalcost2 = 0;

        ZopfliBlockSplitLZ77(options, &lz77,
                             options->blocksplittingmax, &splitpoints2, &npoints2);

        for (i = 0; i <= npoints2; i++)
        {
            size_t start = i == 0 ? 0 : splitpoints2[i - 1];
            size_t end = i == npoints2 ? lz77.size : splitpoints2[i];
            totalcost2 += ZopfliCalculateBlockSizeAutoType(&lz77, start, end);
        }

        if (totalcost2 < totalcost)
        {
            free(splitpoints);
            splitpoints = splitpoints2;
            npoints = npoints2;
        }
        else
        {
            free(splitpoints2);
        }
    }

    for (i = 0; i <= npoints; i++)
    {
        size_t start = i == 0 ? 0 : splitpoints[i - 1];
        size_t end = i == npoints ? lz77.size : splitpoints[i];
        AddLZ77BlockAutoType(options, i == npoints && final,
                             &lz77, start, end, 0,
                             bp, out, outsize);
    }

    ZopfliCleanLZ77Store(&lz77);
    free(splitpoints);
    free(splitpoints_uncompressed);
}

/*
Compresses according to the deflate specification and append the compressed
result to the output.
This function will usually output multiple deflate blocks. If final is 1, then
the final bit will be set on the last block.

options: global program options
btype: the deflate block type. Use 2 for best compression.
-0: non compressed blocks (00)
-1: blocks with fixed tree (01)
-2: blocks with dynamic tree (10)
final: whether this is the last section of the input, sets the final bit to the
last deflate block.
in: the input bytes
insize: number of input bytes
bp: bit pointer for the output array. This must initially be 0, and for
consecutive calls must be reused (it can have values from 0-7). This is
because deflate appends blocks as bit-based data, rather than on byte
boundaries.
out: pointer to the dynamic output array to which the result is appended. Must
be freed after use.
outsize: pointer to the dynamic output array size.
*/
void ZopfliDeflate(const ZopfliOptions *options, int btype, int final,
                   const unsigned char *in, size_t insize,
                   unsigned char *bp, unsigned char **out, size_t *outsize)
{
    size_t offset = *outsize;
    size_t i = 0;
    do
    {
        int masterfinal = (i + ZOPFLI_MASTER_BLOCK_SIZE >= insize);
        int final2 = final && masterfinal;
        size_t size = masterfinal ? insize - i : ZOPFLI_MASTER_BLOCK_SIZE;
        ZopfliDeflatePart(options, btype, final2,
                          in, i, i + size, bp, out, outsize);
        i += size;
    } while (i < insize);
    if (options->verbose)
    {
        fprintf(stderr,
                "Original Size: %lu, Deflate: %lu, Compression: %.2f%%\n",
                (unsigned long)insize, (unsigned long)(*outsize - offset),
                100.0 * (double)(insize - (*outsize - offset)) / (double)insize);
    }
}

void single_test(const unsigned char *in, int btype, int blocksplitting, int blocksplittingmax)
{
    ZopfliOptions options;

    // Configure options
    options.verbose = 1;
    options.verbose_more = 0; // Reduce internal verbose output
    options.numiterations = 15;
    options.blocksplitting = blocksplitting;
    options.blocksplittinglast = 0;
    options.blocksplittingmax = blocksplittingmax;

    unsigned char *out = 0;
    size_t outsize = 0;
    unsigned char bp = 0;
    size_t insize = strlen(in);

    // Perform compression
    ZopfliDeflate(&options, btype, 1, (const unsigned char *)in, insize, &bp, &out, &outsize);
}

void run_all_tests(const unsigned char *in)
{
    single_test(in, 2, 1, 15); // Dynamic Huffman
    single_test(in, 1, 1, 15); // Fixed Huffman
    single_test(in, 0, 1, 15); // Uncompressed

    // Test with different block splitting settings
    single_test(in, 2, 0, 15); // No block splitting
    single_test(in, 2, 1, 5);  // Limited splits
    single_test(in, 2, 1, 0);  // No splits
    single_test(in, 2, 1, 1);  // No splits
    single_test(in, 2, 1, 50); // No splits
    single_test(in, 2, 1, 30); // More splits allowed
}

unsigned char *read_stdin_to_bytes(size_t *out_size)
{
    // Initial buffer size
    size_t buffer_size = 1024;
    size_t total_read = 0;

    // Allocate initial buffer
    unsigned char *buffer = malloc(buffer_size);
    if (buffer == NULL)
    {
        fprintf(stderr, "Memory allocation failed\n");
        return NULL;
    }

    // Read stdin in chunks
    int ch;
    while ((ch = getchar()) != EOF)
    {
        // Resize buffer if needed
        if (total_read >= buffer_size)
        {
            buffer_size *= 2;
            unsigned char *new_buffer = realloc(buffer, buffer_size);
            if (new_buffer == NULL)
            {
                fprintf(stderr, "Memory reallocation failed\n");
                free(buffer);
                return NULL;
            }
            buffer = new_buffer;
        }

        // Store the character
        buffer[total_read++] = (unsigned char)ch;
    }

    // Set the output size
    *out_size = total_read;
    return buffer;
}

int main()
{
    size_t bytes_size;
    unsigned char *bytes = read_stdin_to_bytes(&bytes_size);

    if (bytes == NULL)
    {
        fprintf(stderr, "Failed to read stdin\n");
        return 1;
    }

    // Run tests
    run_all_tests(bytes);

    // Free allocated memory
    free(bytes);

    return 0;
}
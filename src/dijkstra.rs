use core::hash::Hash;
use std::cmp::Ord;
use std::collections::{BinaryHeap, HashMap, HashSet};

pub trait Graph<T: Ord + Hash> {
    fn get_successors(&self, from: &T) -> Vec<(&T, i32)>;
}

impl Graph<usize> for Vec<Vec<(usize, i32)>> {
    fn get_successors(&self, from: &usize) -> Vec<(&usize, i32)> {
        self.get(*from)
            .unwrap()
            .iter()
            .map(|(node, cost)| (node, *cost as i32))
            .collect()
    }
}

pub fn dijkstra<'a, T: Ord + Hash>(
    graph: &'a dyn Graph<T>,
    start: &'a T,
) -> Option<HashMap<&'a T, Vec<&'a T>>> {
    let mut heap: BinaryHeap<(i32, (&T, &T))> = BinaryHeap::new();
    let mut visited: HashSet<&T> = HashSet::new();
    let mut cost_to_reach: HashMap<&T, i32> = HashMap::new();
    let mut predecessors: HashMap<&T, Vec<&T>> = HashMap::new();

    heap.push((0, (start, start)));
    while !heap.is_empty() {
        let (cost, (current, from)) = match heap.pop() {
            Some(infos) => infos,
            None => return None,
        };

        if visited.contains(current) {
            // Maybe ECMP?
            match cost_to_reach.get(current) {
                None => continue,
                Some(optimal_cost) => {
                    if *optimal_cost == cost {
                        // This is ECMP!
                        predecessors.entry(current).or_insert_with(Vec::new).push(from);
                    }
                }
            }
            // Do not need to expand the node, we already did it
            continue;
        }

        visited.insert(current);
        predecessors.entry(current).or_insert_with(Vec::new).push(from);
        cost_to_reach.insert(current, cost);

        // Add all neighbours
        for (neigh, local_cost) in graph
            .get_successors(current)
            .iter()
            .filter(|(neigh, _)| !visited.contains(neigh))
        {
            heap.push((cost - local_cost, (neigh, current)));
        }
    }
    Some(predecessors)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dijkstra_dummy() {
        let mut v: Vec<Vec<(usize, i32)>> = Vec::new();
        v.push(vec![(1, 1)]);
        v.push(vec![(0, 1)]);
        let start: usize = 0;
        let next_hop = dijkstra(&v, &start);
        assert!(next_hop.is_some());
        let nh_unw = next_hop.unwrap();
        assert!(nh_unw.contains_key(&0));
        assert!(nh_unw.contains_key(&1));
        assert!(nh_unw.get(&0).is_some());
        assert_eq!(nh_unw.get(&0).unwrap().len(), 1);
        assert!(nh_unw.get(&1).is_some());
        assert_eq!(nh_unw.get(&1).unwrap().len(), 1);

        assert_eq!(*nh_unw.get(&0).unwrap()[0], 0);
        assert_eq!(*nh_unw.get(&1).unwrap()[0], 0);
    }

    #[test]
    fn test_dijkstra_medium_topo() {
        let mut v: Vec<Vec<(usize, i32)>> = Vec::with_capacity(5);
        v.push(vec![(1, 1), (2, 1)]);
        v.push(vec![(0, 1), (3, 1)]);
        v.push(vec![(0, 1), (3, 2)]);
        v.push(vec![(1, 1), (2, 2), (4, 1)]);
        v.push(vec![(3, 1)]);

        let start: usize = 1;
        let next_hop = dijkstra(&v, &start);
        assert!(next_hop.is_some());
        let nh_unw = next_hop.unwrap();

        let len_paths: Vec<usize> = vec![1; 5];
        let true_next_hops: Vec<usize> = vec![1, 1, 0, 1, 3];

        for i in 0..5 {
            assert!(nh_unw.contains_key(&i));
            assert!(nh_unw.get(&i).is_some());
            assert_eq!(nh_unw.get(&i).unwrap().len(), len_paths[i]);
            assert_eq!(*nh_unw.get(&i).unwrap()[0], true_next_hops[i]);
        }
    }

    #[test]
    fn test_dijkstra_medium_topo_ecmp() {
        let mut v: Vec<Vec<(usize, i32)>> = Vec::with_capacity(5);
        v.push(vec![(1, 1), (2, 1)]);
        v.push(vec![(0, 1), (3, 1)]);
        v.push(vec![(0, 1), (3, 1)]);
        v.push(vec![(1, 1), (2, 1), (4, 1)]);
        v.push(vec![(3, 1)]);

        let start: usize = 0;
        let next_hop = dijkstra(&v, &start);
        assert!(next_hop.is_some());
        let nh_unw = next_hop.unwrap();

        let len_paths: Vec<usize> = vec![1; 5];
        let true_next_hops: Vec<usize> = vec![0, 0, 0, 0, 3];

        for i in 0..5 {
            assert!(nh_unw.contains_key(&i));
            assert!(nh_unw.get(&i).is_some());
            if i == 3 {
                continue; // We will test 3 later
            }
            assert_eq!(nh_unw.get(&i).unwrap().len(), len_paths[i]);
            assert_eq!(*nh_unw.get(&i).unwrap()[0], true_next_hops[i]);
        }

        // Test 3, we should have ECMP there
        assert_eq!(nh_unw.get(&3).unwrap().len(), 2);
        assert!(nh_unw.get(&3).unwrap().contains(&&1));
        assert!(nh_unw.get(&3).unwrap().contains(&&2));
    }
    #[test]
    fn test_dijkstra_house() {
        let mut house: Vec<Vec<(usize, i32)>> = Vec::with_capacity(6);
        house.push(vec![(1, 1), (2, 10)]);
        house.push(vec![(0, 1), (2, 1), (3, 1), (4, 10)]);
        house.push(vec![(0, 10), (1, 1), (4, 1), (5, 1)]);
        house.push(vec![(1, 1), (5, 1)]);
        house.push(vec![(1, 10), (2, 1)]);
        house.push(vec![(2, 1), (3, 1)]);

        let mut spts:Vec<HashMap<usize, Vec<usize>>> = Vec::with_capacity(6);
        spts.push(HashMap::from([
            (0, vec![0]),
            (1, vec![0]),
            (2, vec![1]),
            (3, vec![1]),
            (4, vec![2]),
            (5, vec![2, 3])
        ]));
        spts.push(HashMap::from([
            (0, vec![1]),
            (1, vec![1]),
            (2, vec![1]),
            (3, vec![1]),
            (4, vec![2]),
            (5, vec![2, 3])
        ]));
        spts.push(HashMap::from([
            (0, vec![1]),
            (1, vec![2]),
            (2, vec![2]),
            (3, vec![1, 5]),
            (4, vec![2]),
            (5, vec![2])
        ]));
        spts.push(HashMap::from([
            (0, vec![1]),
            (1, vec![3]),
            (2, vec![1, 5]),
            (3, vec![3]),
            (4, vec![2]),
            (5, vec![3])
        ]));
        spts.push(HashMap::from([
            (0, vec![1]),
            (1, vec![2]),
            (2, vec![4]),
            (3, vec![1, 5]),
            (4, vec![4]),
            (5, vec![2])
        ]));
        spts.push(HashMap::from([
            (0, vec![1]),
            (1, vec![2, 3]),
            (2, vec![5]),
            (3, vec![5]),
            (4, vec![2]),
            (5, vec![5])
        ]));

        for (i, _) in house.iter().enumerate() {
            let spt = dijkstra(&house, &i);
            for (node, parents) in spt.unwrap() {
                let expected_parents = &(&spts)[i][node];
                // same number of parents
                assert_eq!(parents.len(), expected_parents.len());
                // check each parent
                for parent in parents {
                    assert!(&expected_parents.contains(parent));
                }
            }
        }
    }
}

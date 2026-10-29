"""Summarize the saved sequential cold-render comparison (no viewer execution)."""
import json
import math
from pathlib import Path

ROOT = Path(__file__).resolve().parent


def distribution(values):
    ordered = sorted(values)
    return {
        'count': len(values), 'min': ordered[0],
        'median': ordered[(len(ordered) - 1) // 2] if len(ordered) % 2 else
        (ordered[len(ordered) // 2 - 1] + ordered[len(ordered) // 2]) / 2,
        'p95': ordered[math.ceil(len(ordered) * .95) - 1], 'max': ordered[-1],
    }


def validate_pair(before, after):
    def frames(report):
        yield report['first_cold_frame']
        for stage in ['warm_full_range', 'cache_rebuild_same_context', 'cache_rebuild_fresh_context']:
            yield from report[stage]['frames']
        yield from report['zoom_sequence_all_frames']['frames']
    for key in ['sources', 'channel_count', 'cursors', 'viewport', 'full_display_range', 'emergency_preview']:
        assert before[key] == after[key], key
    for left, right in zip(frames(before), frames(after), strict=True):
        for key in ['prepared_rows', 'prepared_channels', 'draw_points', 'shapes', 'tile_builds']:
            assert left[key] == right[key], (key, left[key], right[key])
    assert before['cache_before_first']['line_tiles'] == after['cache_before_first']['line_tiles'] == 0
    assert before['first_cold_frame']['edge_aborts'] == before['source_count']
    assert before['first_cold_frame']['tile_build_workers'] == 1
    assert after['first_cold_frame']['tile_build_workers'] == 4
    for report in [before, after]:
        for frame in frames(report):
            assert frame['tile_build_wall_ms'] <= frame['visible_traces_ms']
            assert 0 <= frame['tile_build_workers'] <= 4
    assert before['cache_after_first'] == after['cache_after_first']
    for report in [before, after]:
        for frame in report['warm_full_range']['frames']:
            assert frame['tile_build_workers'] == frame['tile_builds'] == 0
    assert after['first_cold_frame']['edge_aborts'] == after['source_count']
    assert after['cache_after_first']['step_edge_mib'] == 0


def summarize():
    pairs = [f'4-sources-{i}' for i in range(1, 6)] + ['1-sources', '2-sources', '4-sources-tall']
    for suffix in pairs:
        validate_pair(*[json.loads((ROOT / f'{kind}-{suffix}.json').read_text()) for kind in ['baseline', 'candidate']])
    result = {}
    for kind in ['baseline', 'candidate']:
        reports = [json.loads((ROOT / f'{kind}-4-sources-{i}.json').read_text()) for i in range(1, 6)]
        metrics = ['total_ms', 'ui_ms', 'visible_traces_ms', 'tile_build_ms', 'tile_build_wall_ms', 'edge_scan_ms', 'tessellation_ms']
        item = {
            'first_cold_frame': {metric: distribution([d['first_cold_frame'][metric] for d in reports]) for metric in metrics},
            'rss_after_first_frame_mib': distribution([d['rss_mib']['after_first_frame'] for d in reports]),
            'rss_first_frame_increase_mib': distribution([d['rss_mib']['after_first_frame'] - d['rss_mib']['before_first_frame'] for d in reports]),
            'cache_after_first': reports[0]['cache_after_first'],
            'zoom_operation_ms': {
                str(fraction): distribution([d['zoom_sequence'][index]['operation_frame']['total_ms'] for d in reports])
                for index, fraction in enumerate([.6, .3, .1, .01, 1.0])
            },
        }
        for key in ['warm_full_range', 'cache_rebuild_same_context', 'cache_rebuild_fresh_context']:
            item[key] = distribution([f['total_ms'] for d in reports for f in d[key]['frames']])
        result[kind] = item
    result['validation'] = {'matched_pairs': len(pairs), 'input_geometry_prepared_channels_draw_points_shapes_tile_builds_match': True, 'cold_cache_state_verified': True, 'cache_memory_unchanged': True, 'tile_wall_timing_and_worker_cap_verified': True}
    result['scope'] = 'Cold: five independent process samples per variant. Rebuild and warm distributions pool correlated frames within those same five processes, not independent process samples. All times are milliseconds; RSS is MiB.'
    return result


if __name__ == '__main__':
    output = summarize()
    (ROOT / 'summary.json').write_text(json.dumps(output, indent=2) + '\n')
    for kind in ['baseline', 'candidate']:
        print(kind, json.dumps(output[kind], indent=2))

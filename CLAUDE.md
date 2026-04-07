> 🔴 **HEXA-FIRST**: 모든 코드는 `.hexa`로 작성. 부하 유발 명령 최소화.

> 🔴 **하드코딩 절대 금지**: 상수/도메인/키워드를 코드에 배열로 나열 금지 → `nexus/shared/*.jsonl`에서 동적 로드. 경로는 환경변수+상대경로. 새 항목 추가 = 설정 파일 한 줄, 코드 수정 0.

> 🔴 **NEXUS-6 특이점 연동**: 이 프로젝트의 돌파/발견/실험은 nexus 특이점 사이클 입력이다.
> - **돌파 시**: `HEXA=$HOME/Dev/hexa-lang/target/release/hexa && $HEXA $HOME/Dev/nexus/mk2_hexa/native/blowup.hexa <domain> 3 --no-graph`
> - **발견 기록**: `$HOME/Dev/nexus/shared/growth_bus.jsonl`에 JSON append
> - **전체 상태**: `$HEXA $HOME/Dev/nexus/mk2_hexa/native/command_router.hexa "airgenome 상태"`

# airgenome

> 참조: `shared/absolute_rules.json` → AG1 | `shared/convergence/airgenome.json` | `shared/todo/airgenome.json`

## 육각 투영 6축 (AG1)
CPU/RAM/Swap/Net/Disk/GPU — 15쌍 게이트 → 60바이트 게놈. 모든 프로세스를 6축 시그니처로 투영.

## HEXA-GATE v3.0
kill-free 재해석 전용. 프로세스 패턴 추출 → 사용자 결정 (자동 kill 금지).
게이트 설정: `nexus/shared/gate_config.jsonl`

## Prime Directive
> 모든 프로세스 KILL 없이 성능/자원 개선. 순수 데이터 재해석만.

## 할일 (todo)
- "todo", "할일" → `hexa-bin-actual $HOME/Dev/nexus/mk2_hexa/native/todo.hexa airgenome` 실행 후 **결과를 마크다운 텍스트로 직접 출력** (렌더링되는 표로)

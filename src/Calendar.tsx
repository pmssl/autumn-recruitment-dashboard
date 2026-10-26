import { useMemo, useState } from 'react'
import type { Application } from './main'

const keyOf = (date: Date) => `${date.getFullYear()}-${String(date.getMonth()+1).padStart(2,'0')}-${String(date.getDate()).padStart(2,'0')}`
const validDate = (value?: string) => !!value && /^\d{4}-\d{2}-\d{2}$/.test(value) && keyOf(new Date(value+'T12:00:00')) === value
const labels = { pending: '待进行', completed: '已完成', failed: '已失败' }

export function Calendar({ apps, onOpen }: { apps: Application[]; onOpen: (app: Application) => void }) {
  const today = keyOf(new Date())
  const [month, setMonth] = useState(today.slice(0,7))
  const [selected, setSelected] = useState(today)
  const [pendingOnly, setPendingOnly] = useState(false)
  const [query, setQuery] = useState('')
  const { byDate, undated } = useMemo(() => {
    const byDate = new Map<string, { app: Application; stage: Application['stages'][number] }[]>()
    let undated = 0
    for (const app of apps) {
      if (!`${app.company} ${app.position}`.toLowerCase().includes(query.trim().toLowerCase())) continue
      for (const stage of app.stages) {
        if (pendingOnly && stage.status !== 'pending') continue
        if (!validDate(stage.date)) { undated++; continue }
        const entries = byDate.get(stage.date!) || []
        entries.push({app,stage})
        byDate.set(stage.date!, entries)
      }
    }
    for (const entries of byDate.values()) entries.sort((a,b) => Number(b.stage.status==='pending')-Number(a.stage.status==='pending') || a.app.company.localeCompare(b.app.company,'zh-CN') || a.stage.sortOrder-b.stage.sortOrder)
    return { byDate, undated }
  }, [apps, pendingOnly, query])
  const [year, monthNumber] = month.split('-').map(Number)
  const first = new Date(year, monthNumber-1, 1)
  const offset = (first.getDay()+6)%7
  const days = new Date(year,monthNumber,0).getDate()
  const cellCount = Math.ceil((offset+days)/7)*7
  const selectedEntries = byDate.get(selected) || []
  const monthCount = [...byDate].filter(([date])=>date.startsWith(month)).reduce((sum,[,entries])=>sum+entries.length,0)
  const changeMonth = (value: string) => {
    if (!/^\d{4}-\d{2}$/.test(value) || Number(value.slice(0,4))<1900 || Number(value.slice(0,4))>2100) return
    setMonth(value); setSelected(value===today.slice(0,7)?today:value+'-01')
  }
  const moveMonth = (delta: number) => changeMonth(keyOf(new Date(year,monthNumber-1+delta,1)).slice(0,7))
  return <main className="calendar-page">
    <div className="calendar-toolbar">
      <div className="calendar-month-controls"><button aria-label="上个月" onClick={()=>moveMonth(-1)}>‹</button><label><span className="sr-only">选择月份</span><input aria-label="选择月份" type="month" min="1900-01" max="2100-12" value={month} onChange={e=>changeMonth(e.target.value)}/></label><button aria-label="下个月" onClick={()=>moveMonth(1)}>›</button><button onClick={()=>{setMonth(today.slice(0,7));setSelected(today)}}>今天</button></div>
      <input className="calendar-search" aria-label="日历搜索公司或岗位" placeholder="搜索公司 / 岗位" value={query} onChange={e=>setQuery(e.target.value)}/>
      <label className="calendar-pending"><input type="checkbox" checked={pendingOnly} onChange={e=>setPendingOnly(e.target.checked)}/>仅看未完成进展</label>
    </div>
    <div className="calendar-summary"><span>本月 {monthCount} 项进展</span><div className="calendar-legend"><span className="pending">○ 待进行</span><span className="completed">✓ 已完成</span><span className="failed">× 已失败</span></div></div>
    <div className="calendar-layout"><section className="calendar-board" aria-label={`${year}年${monthNumber}月进展日历`}>
      <div className="calendar-weekdays">{['一','二','三','四','五','六','日'].map(day=><span key={day}>周{day}</span>)}</div>
      <div className="calendar-grid">{Array.from({length:cellCount},(_,i)=>{
        const date=keyOf(new Date(year,monthNumber-1,i-offset+1))
        const entries=byDate.get(date)||[]
        return <button key={date} data-date={date} className={`calendar-day ${date.startsWith(month)?'':'outside'} ${entries.length?'has-events':''} ${selected===date?'selected':''} ${date===today?'today':''}`} aria-pressed={selected===date} aria-label={`${date}，${entries.length}项进展${date===today?'，今天':''}`} onClick={()=>{setSelected(date);if(!date.startsWith(month))setMonth(date.slice(0,7))}}>
          <span className="calendar-day-number">{Number(date.slice(-2))}{date===today&&<small>今天</small>}{entries.length>0&&<b>{entries.length}</b>}</span>
          <span className="calendar-previews">{entries.slice(0,2).map(({app,stage})=><span key={app.id+stage.id} className={`calendar-preview ${stage.status}`} title={`${app.company} / ${app.position} · ${stage.title} · ${labels[stage.status]}`}>{stage.status==='pending'?'○':stage.status==='completed'?'✓':'×'} {app.company} · {stage.title}</span>)}{entries.length>2&&<small>另有 {entries.length-2} 项</small>}</span>
        </button>
      })}</div>
    </section><aside className="calendar-agenda" aria-live="polite"><h2>{selected.replaceAll('-',' / ')}</h2><p>{selectedEntries.length} 项进展 · 点击查看投递详情</p>
      {selectedEntries.length?selectedEntries.map(({app,stage})=><button className="calendar-event" key={app.id+stage.id} onClick={()=>onOpen(app)}><span className={`calendar-event-status ${stage.status}`}>{labels[stage.status]}</span><strong>{stage.title}</strong><span>{app.company}</span><small>{app.position}</small>{stage.notes&&<p>{stage.notes}</p>}</button>):<div className="calendar-empty">当天没有符合条件的进展</div>}
    </aside></div>
    <p className="calendar-footnote">按进展中填写的日期显示，含投递、笔试、面试等节点。{undated>0&&`${undated} 项进展未填写有效日期，暂不显示在日历中。`}导入记录中的日期若为截止日期，请以原文备注为准。</p>
  </main>
}

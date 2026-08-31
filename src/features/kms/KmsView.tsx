/**
 * KMS —— 知识管理系统视图。
 *
 * Phase 3 条目 12：基于 grep 的 wiki + /dream 会话挖掘。
 * 列出知识库，支持创建/编辑/搜索页面。
 */
import { useEffect, useState } from 'react';
import { BookOpen, Plus, Search, Trash2, FileText } from 'lucide-react';
import { useQuery, useMutation, useQueryClient } from '@tanstack/react-query';
import { PageShell } from '@/features/shell/PageShell';
import { Card, Badge, Icon, Button, EmptyState, Spinner, Input } from '@/features/design-system';
import { useI18n } from '@/utils/i18n';
import { confirmDialog } from '@/features/modals/ConfirmDialog';
import {
  reflect_kms_list,
  reflect_kms_create,
  reflect_kms_delete,
  reflect_kms_list_pages,
  reflect_kms_save_page,
  reflect_kms_search,
  type ReflectWikiInfo,
  type ReflectPage,
  type ReflectSearchResult,
} from '@/utils/commands';
import s from './KmsView.module.css';

export function KmsView() {
  const queryClient = useQueryClient();
  const { t } = useI18n();
  const [selectedWiki, setSelectedWiki] = useState<string | null>(null);
  const [showCreateForm, setShowCreateForm] = useState(false);
  const [newWikiName, setNewWikiName] = useState('');
  const [newWikiDesc, setNewWikiDesc] = useState('');
  // 搜索输入与查询分离：防抖后才触发跨 wiki grep，避免每个按键一次后端请求。
  const [searchInput, setSearchInput] = useState('');
  const [searchQuery, setSearchQuery] = useState('');
  const [editingPage, setEditingPage] = useState<ReflectPage | null>(null);
  const [pageContent, setPageContent] = useState('');
  const [pageName, setPageName] = useState('');

  useEffect(() => {
    const timer = setTimeout(() => setSearchQuery(searchInput), 200);
    return () => clearTimeout(timer);
  }, [searchInput]);

  const wikisQuery = useQuery({
    queryKey: ['kms', 'wikis'],
    queryFn: reflect_kms_list,
  });

  const pagesQuery = useQuery({
    queryKey: ['kms', 'pages', selectedWiki],
    queryFn: () => reflect_kms_list_pages(selectedWiki!),
    enabled: !!selectedWiki,
  });

  const searchResults = useQuery({
    queryKey: ['kms', 'search', searchQuery],
    queryFn: () => reflect_kms_search(searchQuery),
    enabled: searchQuery.length > 1,
  });

  const createWiki = useMutation({
    mutationFn: () => reflect_kms_create(newWikiName, newWikiDesc || null),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['kms', 'wikis'] });
      setShowCreateForm(false);
      setNewWikiName('');
      setNewWikiDesc('');
    },
  });

  const deleteWiki = useMutation({
    mutationFn: (name: string) => reflect_kms_delete(name),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['kms', 'wikis'] });
      if (selectedWiki) setSelectedWiki(null);
    },
  });

  const savePage = useMutation({
    mutationFn: () =>
      reflect_kms_save_page(selectedWiki!, pageName, pageContent, null, null),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['kms', 'pages', selectedWiki] });
      setEditingPage(null);
      setPageContent('');
      setPageName('');
    },
  });

  return (
    <PageShell
      icon={BookOpen}
      title={t('kms.title')}
      subtitle={t('kms.subtitle')}
      width="lg"
    >
      {/* Search bar */}
      <Card level="flat" padding="md" className={s.searchCard}>
        <div className={s.searchRow}>
          <Icon icon={Search} size={16} />
          <Input
            placeholder={t('kms.searchPlaceholder')}
            value={searchInput}
            onChange={(e: React.ChangeEvent<HTMLInputElement>) => setSearchInput(e.target.value)}
            className={s.searchInput}
          />
        </div>
        {searchResults.data && searchResults.data.length > 0 && (
          <div className={s.searchResults}>
            {searchResults.data.map((r: ReflectSearchResult, i: number) => (
              <div key={i} className={s.searchItem}>
                <Badge variant="info">{r.wiki}/{r.page}</Badge>
                <span className={s.snippet}>{r.lines[0]}</span>
              </div>
            ))}
          </div>
        )}
      </Card>

      {/* Wiki selector */}
      <div className={s.wikiBar}>
        {wikisQuery.data?.map((wiki: ReflectWikiInfo) => (
          <button
            key={wiki.name}
            type="button"
            className={s.wikiTab}
            data-active={selectedWiki === wiki.name || undefined}
            onClick={() => setSelectedWiki(wiki.name)}
          >
            {wiki.name}
            <Badge variant="neutral">{wiki.pageCount}</Badge>
          </button>
        ))}
        <Button variant="ghost" size="sm" onClick={() => setShowCreateForm(!showCreateForm)}>
          <Icon icon={Plus} size={14} /> {t('kms.new')}
        </Button>
      </div>

      {/* Create wiki form */}
      {showCreateForm && (
        <Card level="outlined" padding="md" className={s.createForm}>
          <Input
            placeholder={t('kms.wikiNamePlaceholder')}
            value={newWikiName}
            onChange={(e: React.ChangeEvent<HTMLInputElement>) => setNewWikiName(e.target.value)}
          />
          <Input
            placeholder={t('kms.descriptionPlaceholder')}
            value={newWikiDesc}
            onChange={(e: React.ChangeEvent<HTMLInputElement>) => setNewWikiDesc(e.target.value)}
          />
          <Button
            variant="primary"
            size="sm"
            onClick={() => createWiki.mutate()}
            disabled={!newWikiName || createWiki.isPending}
          >
            {t('kms.create')}
          </Button>
        </Card>
      )}

      {/* Selected wiki content */}
      {selectedWiki ? (
        <div className={s.wikiContent}>
          <div className={s.wikiHeader}>
            <h2 className={s.wikiTitle}>{selectedWiki}</h2>
            <div className={s.wikiActions}>
              <Button
                variant="ghost"
                size="sm"
                onClick={() => {
                  // 打开空白编辑器（此前只会 setEditingPage(null)，编辑器
                  // 永远打不开，页面无法创建/编辑）。
                  setEditingPage({
                    name: '',
                    content: '',
                    tags: [],
                    wiki: selectedWiki ?? '',
                    title: null,
                    modifiedAtMs: 0,
                  });
                  setPageName('');
                  setPageContent('');
                }}
              >
                <Icon icon={Plus} size={14} /> {t('kms.newPage')}
              </Button>
              <Button
                variant="ghost"
                size="sm"
                onClick={async () => {
                  // remove_dir_all 级别的破坏性操作，必须二次确认。
                  const ok = await confirmDialog({
                    title: t('kms.deleteWiki'),
                    message: t('kms.deleteWikiConfirm', { name: selectedWiki }),
                  });
                  if (ok) deleteWiki.mutate(selectedWiki);
                }}
              >
                <Icon icon={Trash2} size={14} /> {t('kms.deleteWiki')}
              </Button>
            </div>
          </div>

          {editingPage !== null && (
            <Card level="outlined" padding="md" className={s.editorCard}>
              <Input
                placeholder={t('kms.pageNamePlaceholder')}
                value={pageName}
                onChange={(e: React.ChangeEvent<HTMLInputElement>) => setPageName(e.target.value)}
              />
              <textarea
                className={s.editor}
                placeholder={t('kms.pageContentPlaceholder')}
                value={pageContent}
                onChange={(e) => setPageContent(e.target.value)}
                rows={10}
              />
              <div className={s.editorActions}>
                <Button
                  variant="primary"
                  size="sm"
                  onClick={() => savePage.mutate()}
                  disabled={!pageName || savePage.isPending}
                >
                  {t('common.save')}
                </Button>
                <Button variant="ghost" size="sm" onClick={() => setEditingPage(null)}>
                  {t('common.cancel')}
                </Button>
              </div>
            </Card>
          )}

          {pagesQuery.isLoading ? (
            <Spinner size={20} />
          ) : pagesQuery.data?.length === 0 ? (
            <EmptyState
              icon={<Icon icon={FileText} />}
              title={t('kms.noPages')}
              description={t('kms.noPagesDesc')}
            />
          ) : (
            <div className={s.pageList}>
              {pagesQuery.data?.map((page: ReflectPage) => (
                <Card
                  key={page.name}
                  level="outlined"
                  padding="md"
                  className={s.pageCard}
                  role="button"
                  tabIndex={0}
                  title={t('kms.editPage')}
                  onClick={() => {
                    setEditingPage(page);
                    setPageName(page.name);
                    setPageContent(page.content);
                  }}
                  onKeyDown={(e: React.KeyboardEvent) => {
                    if (e.key === 'Enter' || e.key === ' ') {
                      setEditingPage(page);
                      setPageName(page.name);
                      setPageContent(page.content);
                    }
                  }}
                >
                  <div className={s.pageHeader}>
                    <Icon icon={FileText} size={16} />
                    <span className={s.pageName}>{page.name}</span>
                    {page.tags.map((tag) => (
                      <Badge key={tag} variant="neutral">{tag}</Badge>
                    ))}
                  </div>
                  <pre className={s.pageContent}>{page.content.slice(0, 200)}</pre>
                </Card>
              ))}
            </div>
          )}
        </div>
      ) : wikisQuery.data?.length === 0 ? (
        <EmptyState
          icon={<Icon icon={BookOpen} />}
          title={t('kms.noWikis')}
          description={t('kms.noWikisDesc')}
        />
      ) : wikisQuery.isLoading ? (
        <Spinner size={20} />
      ) : null}
    </PageShell>
  );
}